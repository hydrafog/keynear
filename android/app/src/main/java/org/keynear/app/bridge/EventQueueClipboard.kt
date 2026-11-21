package org.keynear.app.bridge

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.util.Log
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.data.DeviceRepo
import org.keynear.app.data.PairingRepo
import org.keynear.app.data.TransferRepo
import org.keynear.app.share.DirectShareManager
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.validateDisplayNameInput
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiClipboardItem
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

internal fun EventQueue.syncClipboardPolicyToNode(node: OrionNodeHandle? = orionNode) {
    val activeNode = node ?: return
    val cfg = cachedDaemonConfig
    runCatching {
        activeNode.setClipboardConfig(
            clipboardSyncEnabled = cfg.clipboardSyncEnabled,
            clipboardImagesEnabled = cfg.clipboardImagesEnabled,
            otpPassthroughEnabled = cfg.otpPassthroughEnabled,
            ignoredMimeTypes = cfg.ignoredMimeTypes,
        )
    }
}
internal fun EventQueue.syncClipboardStates(context: Context): Boolean {
    val app = context.applicationContext
    attachContext(app)
    val node = orionNode ?: return false
    syncClipboardPolicyToNode(node)
    val cfg = cachedDaemonConfig
    if (!cfg.clipboardSyncEnabled) return false

    val incoming = runCatching { node.pollIncomingClipboard() }.getOrNull() ?: return false
    if (incoming.isEmpty()) return false

    var applied = false
    for (item in incoming) {
        if (applyIncomingClipboardItem(app, node, item, cfg)) {
            applied = true
        }
    }
    return applied
}

private fun EventQueue.applyIncomingClipboardItem(
    context: Context,
    node: OrionNodeHandle,
    item: FfiClipboardItem,
    cfg: DaemonConfigUi,
): Boolean {
    val mimesToCheck = if (item.offeredMimes.isNotEmpty()) {
        item.offeredMimes
    } else {
        listOf(item.mimeType)
    }
    if (shouldIgnoreClipboardMimes(mimesToCheck, cfg.ignoredMimeTypes)) {
        return false
    }
    val resolved: FfiClipboardItem = if (item.isImageOffer) {
        if (!cfg.clipboardImagesEnabled) return false
        runCatching {
            node.fetchClipboardOffer(item.peerDeviceId, item.offerId, item.mimeType)
        }.getOrNull() ?: return false
    } else {
        item
    }
    if (shouldIgnoreClipboardMimes(listOf(resolved.mimeType), cfg.ignoredMimeTypes)) {
        return false
    }
    return applyIncomingClipboardToAndroid(context, resolved)
}
internal fun EventQueue.applyIncomingClipboardToAndroid(context: Context, item: FfiClipboardItem): Boolean {
    val mainHandler = Handler(Looper.getMainLooper())
    val mime = item.mimeType.lowercase(Locale.ROOT)
    if (mime.startsWith("image/")) {
        return applyIncomingImageClipboard(context, item, mime, mainHandler)
    }

    val text = item.data.toString(Charsets.UTF_8)
    if (text.isEmpty()) return false
    return mainHandler.post {
        runCatching {
            val manager = context.getSystemService(ClipboardManager::class.java)
            manager?.setPrimaryClip(ClipData.newPlainText("Keynear", text))
        }
    }
}

private fun EventQueue.applyIncomingImageClipboard(
    context: Context,
    item: FfiClipboardItem,
    mime: String,
    mainHandler: Handler,
): Boolean {
    if (!cachedDaemonConfig.clipboardImagesEnabled || item.data.isEmpty()) return false
    val ext = imageExtensionForMime(mime)
    val uri = writeClipboardImageToMediaStore(context, item, ext) ?: return false
    return mainHandler.post {
        runCatching {
            val manager = context.getSystemService(ClipboardManager::class.java)
            manager?.setPrimaryClip(ClipData.newUri(context.contentResolver, "Keynear image", uri))
        }
    }
}

private fun imageExtensionForMime(mime: String): String = when {
    mime.contains("png") -> "png"
    mime.contains("webp") -> "webp"
    mime.contains("gif") -> "gif"
    else -> "jpg"
}

private fun writeClipboardImageToMediaStore(context: Context, item: FfiClipboardItem, ext: String): Uri? {
    val values = ContentValues().apply {
        put(MediaStore.Images.Media.DISPLAY_NAME, "keynear-clip-${System.currentTimeMillis()}.$ext")
        put(MediaStore.Images.Media.MIME_TYPE, item.mimeType.substringBefore(';').trim())
        put(MediaStore.Images.Media.RELATIVE_PATH, Environment.DIRECTORY_PICTURES + "/Keynear")
        put(MediaStore.Images.Media.IS_PENDING, 1)
    }
    val uri = runCatching {
        context.contentResolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values)
    }.getOrNull() ?: return null
    val wrote = runCatching {
        context.contentResolver.openOutputStream(uri)?.use { out ->
            out.write(item.data)
        }
        val clearPending = ContentValues().apply { put(MediaStore.Images.Media.IS_PENDING, 0) }
        context.contentResolver.update(uri, clearPending, null, null)
    }.isSuccess
    if (!wrote) {
        runCatching { context.contentResolver.delete(uri, null, null) }
        return null
    }
    return uri
}

internal fun EventQueue.sendPhoneClipboardTextImpl(text: String): Result<Unit> {
    val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
    if (!cachedDaemonConfig.clipboardSyncEnabled) {
        return Result.failure(IllegalStateException("Clipboard sync is disabled"))
    }
    if (shouldIgnoreClipboardMimes(listOf("text/plain"), cachedDaemonConfig.ignoredMimeTypes)) {
        return Result.failure(IllegalStateException("Clipboard MIME type is ignored"))
    }
    val node = ensureOrionNode(ctx)
        ?: return Result.failure(IllegalStateException("Orion transport node unavailable"))
    syncClipboardPolicyToNode(node)
    syncAllPairedPeersToOrion(node)
    return runCatching {
        node.sendClipboardText(text, false)
    }
}
internal fun EventQueue.sendPhoneClipboardBlobImpl(mimeType: String, data: ByteArray): Result<Unit> {
    val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
    if (!cachedDaemonConfig.clipboardSyncEnabled || !cachedDaemonConfig.clipboardImagesEnabled) {
        return Result.failure(IllegalStateException("Clipboard image sync is disabled"))
    }
    if (shouldIgnoreClipboardMimes(listOf(mimeType), cachedDaemonConfig.ignoredMimeTypes)) {
        return Result.failure(IllegalStateException("Clipboard MIME type is ignored"))
    }
    val node = ensureOrionNode(ctx)
        ?: return Result.failure(IllegalStateException("Orion transport node unavailable"))
    syncClipboardPolicyToNode(node)
    syncAllPairedPeersToOrion(node)
    return runCatching {
        node.sendClipboardBlob(mimeType, data)
    }
}
internal fun EventQueue.sendVerifiedOtpImpl(code: String): Result<Unit> {
    val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
    if (!cachedDaemonConfig.otpPassthroughEnabled) {
        return Result.failure(IllegalStateException("OTP pass-through is disabled"))
    }
    val node = ensureOrionNode(ctx)
        ?: return Result.failure(IllegalStateException("Orion transport node unavailable"))
    syncClipboardPolicyToNode(node)
    syncAllPairedPeersToOrion(node)
    return runCatching {
        node.sendClipboardText(code, true)
    }
}
