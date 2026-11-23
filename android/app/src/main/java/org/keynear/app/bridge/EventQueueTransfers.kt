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
import org.keynear.app.data.OutboundTarget
import org.keynear.app.data.PairingRepo
import org.keynear.app.data.TransferRepo
import org.keynear.app.data.TransferSyncEnv
import org.keynear.app.data.TransferSyncHandler
import org.keynear.app.data.TransferSyncIds
import org.keynear.app.data.TransferSyncInput
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

internal fun EventQueue.syncTransferStates(context: Context): Boolean {
    val app = context.applicationContext
    attachContext(app)
    val node = orionNode ?: return false
    return TransferSyncHandler.syncTransferStates(
        TransferSyncInput(
            env = TransferSyncEnv(
                app = app,
                node = node,
                pairedDevices = getPairedDevices(),
                incomingDir = File(app.filesDir, "incoming"),
            ),
            tracker = mediaStoreTracker,
            ids = TransferSyncIds(
                fdHandedOffIds = fdHandedOffIds,
                exportedArchiveIds = exportedArchiveIds,
                notifiedConsentIds = notifiedConsentIds,
                notifiedTerminalIds = notifiedTerminalIds,
            ),
        ),
    )
}
internal fun EventQueue.acceptIncomingTransferOrSync(context: Context, transferId: String): CoreState {
    attachContext(context)
    return acceptTransfer(transferId)
}
internal fun EventQueue.stageFileUrisImpl(uris: List<String>): CoreState {
    val ctx = appContext
    uris.forEach { raw ->
        if (raw.isBlank()) return@forEach
        if (ctx != null) {
            val (name, size, mime) = TransferRepo.queryContentUriMetadata(ctx, Uri.parse(raw))
            sendQueueStore.addFile(name, size, mime, raw)
        } else {
            val name = raw.substringAfterLast('/').ifBlank { "shared-file.bin" }
            sendQueueStore.addFile(name, 0L, "application/octet-stream", raw)
        }
    }
    return buildState()
}
internal fun EventQueue.stageFolderUriImpl(
    folderUri: String,
    displayName: String,
    sizeBytes: Long,
): CoreState {
    if (folderUri.isNotBlank()) {
        sendQueueStore.addFolder(displayName, folderUri, sizeBytes)
    }
    return buildState()
}
internal fun EventQueue.sendQueueToPeerImpl(peerDeviceId: String): Result<TransferUi> {
    val items = sendQueueStore.snapshot()
    if (items.isEmpty()) {
        return Result.failure(IllegalArgumentException("Stage at least one file, folder, or snippet first"))
    }
    val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
    val node = ensureOrionNode(ctx)
        ?: return Result.failure(IllegalStateException("Local transfer node could not start"))
    syncAllPairedPeersToOrion(node)

    val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
    val target = OutboundTarget(
        context = ctx,
        node = node,
        peerDeviceId = peerDeviceId,
        peerNames = peerNames,
    )
    val result = TransferRepo.sendQueueItemsViaOrion(target, items)
    if (result.isSuccess) {
        sendQueueStore.clear()
    }
    return result
}
internal fun EventQueue.acceptTransferImpl(transferId: String): CoreState {
    val node = orionNode ?: return CoreState.Error("Orion transfer node is not running", snapshotFromCurrent())
    return try {
        val session = node.respondConsent(transferId, true)
        appContext?.let { PairingNotifier.cancelTransferNotification(it, transferId) }
        if (TransferRepo.isSingleFileReceive(session)) {
            val handedOff = TransferRepo.prepareAndHandOffReceiveFd(mediaStoreTracker, fdHandedOffIds, node, session)
            if (!handedOff) {
                runCatching { node.cancelTransfer(transferId) }
                return CoreState.Error("Unable to open MediaStore download output", snapshotFromCurrent())
            }
        }
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to accept transfer", snapshotFromCurrent())
    }
}
internal fun EventQueue.declineTransferImpl(transferId: String): CoreState {
    val node = orionNode ?: return buildState()
    return try {
        node.respondConsent(transferId, false)
        appContext?.let { PairingNotifier.cancelTransferNotification(it, transferId) }
        mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to decline transfer", snapshotFromCurrent())
    }
}
internal fun EventQueue.cancelTransferImpl(transferId: String): CoreState {
    val node = orionNode ?: return buildState()
    return try {
        node.cancelTransfer(transferId)
        appContext?.let { PairingNotifier.cancelTransferNotification(it, transferId) }
        mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to cancel transfer", snapshotFromCurrent())
    }
}
internal fun EventQueue.sendContentUrisImpl(
    peerDeviceId: String,
    uris: List<String>,
    sharedText: String?,
): Result<TransferUi> {
    val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
    val node = ensureOrionNode(ctx)
        ?: return Result.failure(IllegalStateException("Local transfer node could not start"))
    syncAllPairedPeersToOrion(node)

    val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
    val target = OutboundTarget(
        context = ctx,
        node = node,
        peerDeviceId = peerDeviceId,
        peerNames = peerNames,
    )
    return TransferRepo.sendContentUrisViaOrion(
        target = target,
        uris = uris,
        sharedText = sharedText,
    )
}
