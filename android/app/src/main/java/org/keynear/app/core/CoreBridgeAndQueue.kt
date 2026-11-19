package org.keynear.app.core

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.MediaStore
import java.io.File
import java.security.SecureRandom
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

interface CoreBridge {
    fun loadState(): CoreState
    fun cycleVisibility(): CoreState
    fun setVisibility(mode: VisibilityState): CoreState = loadState()
    fun tick(): CoreState = loadState()
    fun updateNearbyPeers(peers: List<DiscoveredPeerUi>) {}
    fun updateBluetoothStatus(status: BluetoothStatus, warning: String? = null) {}
    fun updateLocalNetworkStatus(status: LocalNetworkStatus, message: String? = null) {}
    fun currentLocalNetworkStatus(): LocalNetworkStatus = LocalNetworkStatus.READY
    fun setPendingPairing(prompt: PendingPairingUi?) {}
    fun unpairDevice(deviceId: String): CoreState = loadState()
    fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState = loadState()
    fun setAutoAcceptFiles(deviceId: String, enabled: Boolean): CoreState = loadState()
    fun resetBleBond(deviceId: String): CoreState = loadState()
    fun acceptTransfer(transferId: String): CoreState = loadState()
    fun declineTransfer(transferId: String): CoreState = loadState()
    fun cancelTransfer(transferId: String): CoreState = loadState()
    fun stageFileUris(uris: List<String>): CoreState = loadState()
    fun stageFolderUri(
        folderUri: String,
        displayName: String,
        fileUris: List<String> = emptyList(),
    ): CoreState = loadState()
    fun stageClipboardText(text: String): CoreState = loadState()
    fun stageTextSnippet(text: String): CoreState = loadState()
    fun removeSendQueueItem(itemId: String): CoreState = loadState()
    fun clearSendQueue(): CoreState = loadState()
    fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> =
        Result.failure(UnsupportedOperationException("send unavailable"))
    fun updateDaemonConfig(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState = loadState()
    fun sendPhoneClipboardText(text: String): Result<Unit> =
        Result.failure(UnsupportedOperationException("clipboard send unavailable"))
    fun sendPhoneClipboardBlob(mimeType: String, data: ByteArray): Result<Unit> =
        Result.failure(UnsupportedOperationException("clipboard blob unavailable"))
    fun sendVerifiedOtp(code: String): Result<Unit> =
        Result.failure(UnsupportedOperationException("otp send unavailable"))
    fun sendContentUris(
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> = Result.failure(UnsupportedOperationException("send unavailable"))
}

interface PendingRowOps<T> {
    fun createPending(fileName: String, mimeType: String): T?
    fun openDetachedFd(handle: T): Int?
    fun clearPendingFlag(handle: T)
    fun deleteRow(handle: T)
}

class PendingMediaStoreTracker<T>(
    private val ops: PendingRowOps<T>,
) {
    private val pendingRows = ConcurrentHashMap<String, T>()

    fun prepareAndHandOff(
        transferId: String,
        fileName: String,
        mimeType: String,
        handOffFd: (Int) -> Unit,
    ): Boolean {
        val handle = ops.createPending(fileName, mimeType) ?: return false
        var handedOff = false
        try {
            val fd = ops.openDetachedFd(handle) ?: return false
            handOffFd(fd)
            handedOff = true
            pendingRows[transferId] = handle
            return true
        } finally {
            if (!handedOff) {
                ops.deleteRow(handle)
            }
        }
    }

    fun reconcileState(transferId: String, state: TransferStateUi) {
        when (state) {
            TransferStateUi.COMPLETED -> pendingRows.remove(transferId)?.let { ops.clearPendingFlag(it) }
            TransferStateUi.CANCELLED, TransferStateUi.FAILED ->
                pendingRows.remove(transferId)?.let { ops.deleteRow(it) }
            TransferStateUi.PENDING_CONSENT, TransferStateUi.ACTIVE -> Unit
        }
    }

    fun hasPending(transferId: String): Boolean = pendingRows.containsKey(transferId)

    fun cleanupAllPending() {
        val keys = pendingRows.keys.toList()
        for (key in keys) {
            pendingRows.remove(key)?.let { ops.deleteRow(it) }
        }
    }
}

internal class AndroidMediaStoreOps(
    private val context: Context,
) : PendingRowOps<Uri> {
    override fun createPending(fileName: String, mimeType: String): Uri? {
        val cleanName = sanitizeFileName(fileName)
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, cleanName)
            put(MediaStore.Downloads.MIME_TYPE, mimeType.ifBlank { "application/octet-stream" })
            put(MediaStore.Downloads.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        return runCatching {
            context.contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
        }.getOrNull()
    }

    override fun openDetachedFd(handle: Uri): Int? = runCatching {
        val pfd: ParcelFileDescriptor = context.contentResolver.openFileDescriptor(handle, "w")
            ?: return@runCatching null
        pfd.detachFd()
    }.getOrNull()

    override fun clearPendingFlag(handle: Uri) {
        val values = ContentValues().apply {
            put(MediaStore.Downloads.IS_PENDING, 0)
        }
        runCatching {
            context.contentResolver.update(handle, values, null, null)
        }
    }

    override fun deleteRow(handle: Uri) {
        runCatching {
            context.contentResolver.delete(handle, null, null)
        }
    }

    private fun sanitizeFileName(raw: String): String {
        val base = raw.substringAfterLast('/').substringAfterLast('\\').trim()
        return base.ifEmpty { "keynear-download.bin" }
    }
}

class VisibilityCountdownTracker(
    private val timeoutSecs: Long = 600L,
) {
    private var mode: VisibilityState = VisibilityState.PAIRED_ONLY
    private var remainingSecs: Long = 0L

    @Synchronized
    fun currentMode(): VisibilityState = mode

    @Synchronized
    fun remainingSecs(): Long = remainingSecs

    @Synchronized
    fun setMode(next: VisibilityState) {
        mode = next
        remainingSecs = if (next == VisibilityState.EVERYONE_NEARBY) timeoutSecs else 0L
    }

    @Synchronized
    fun cycle(): VisibilityState {
        setMode(mode.next())
        return mode
    }

    @Synchronized
    fun tick(): VisibilityState {
        if (mode == VisibilityState.EVERYONE_NEARBY) {
            if (remainingSecs <= 1L) {
                mode = VisibilityState.PAIRED_ONLY
                remainingSecs = 0L
            } else {
                remainingSecs -= 1L
            }
        }
        return mode
    }
}

private val ALLOWED_SHARE_SCHEMES = setOf("content", "file")

fun validateShareIntent(
    action: String?,
    streamUris: List<String>,
    sharedText: String?,
): ShareValidationResult {
    if (action != "android.intent.action.SEND" && action != "android.intent.action.SEND_MULTIPLE") {
        return ShareValidationResult.Invalid("unsupported_action")
    }

    val normalizedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
    if (streamUris.isEmpty() && normalizedText == null) {
        return ShareValidationResult.Invalid("missing_payload")
    }

    for (rawUri in streamUris) {
        val trimmed = rawUri.trim()
        val colonIndex = trimmed.indexOf(':')
        if (colonIndex <= 0) {
            return ShareValidationResult.Invalid("missing_uri_scheme")
        }
        val scheme = trimmed.substring(0, colonIndex).lowercase()
        if (scheme !in ALLOWED_SHARE_SCHEMES) {
            return ShareValidationResult.Invalid("disallowed_uri_scheme")
        }
    }

    return ShareValidationResult.Valid(
        itemCount = streamUris.size,
        hasText = normalizedText != null,
    )
}

fun formatByteCount(bytes: Long): String {
    val safe = bytes.coerceAtLeast(0L)
    if (safe < 1024L) return "$safe B"
    val kib = safe / 1024.0
    if (kib < 1024.0) return "%.1f KB".format(kib)
    val mib = kib / 1024.0
    if (mib < 1024.0) return "%.1f MB".format(mib)
    val gib = mib / 1024.0
    return "%.2f GB".format(gib)
}

fun evaluateConsentRequirement(
    isPaired: Boolean,
    deviceAutoAccept: Boolean,
    globalAutoAccept: Boolean,
): Boolean = isPaired && (deviceAutoAccept || globalAutoAccept)

class SendQueueStore {
    private val items = mutableListOf<SendQueueItemUi>()
    private val seq = AtomicLong(1L)

    @Synchronized
    fun snapshot(): List<SendQueueItemUi> = items.toList()

    @Synchronized
    fun addFile(
        name: String,
        sizeBytes: Long,
        mime: String,
        uriString: String,
    ): SendQueueItemUi {
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = name.ifBlank { "shared-file.bin" },
            sizeBytes = sizeBytes.coerceAtLeast(0L),
            kind = SendItemKind.FILE,
            mime = mime.ifBlank { "application/octet-stream" },
            uriString = uriString,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addFolder(
        displayName: String,
        uriString: String,
        sizeBytes: Long = 0L,
    ): SendQueueItemUi {
        val clean = displayName.trim().ifBlank { "Folder" }
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "$clean/",
            sizeBytes = sizeBytes.coerceAtLeast(0L),
            kind = SendItemKind.FOLDER,
            mime = "inode/directory",
            uriString = uriString,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addClipboard(text: String): SendQueueItemUi? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        val bytes = clean.toByteArray(Charsets.UTF_8).size.toLong()
        val label = if (clean.length > 36) "${clean.take(36)}..." else clean
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "Clipboard: $label",
            sizeBytes = bytes,
            kind = SendItemKind.CLIPBOARD,
            mime = "text/plain",
            textPayload = clean,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addTextSnippet(text: String): SendQueueItemUi? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        val bytes = clean.toByteArray(Charsets.UTF_8).size.toLong()
        val label = if (clean.length > 32) "${clean.take(32)}..." else clean
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "Text: $label",
            sizeBytes = bytes,
            kind = SendItemKind.TEXT,
            mime = "text/plain",
            textPayload = clean,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun remove(itemId: String) {
        items.removeAll { it.id == itemId }
    }

    @Synchronized
    fun clear() {
        items.clear()
    }
}


object EndpointHexCodec {
    private const val ENDPOINT_ID_HEX_LEN = 64

    fun decode32(hex: String): ByteArray? {
        val trimmed = hex.trim()
        if (trimmed.length != ENDPOINT_ID_HEX_LEN) return null
        val out = ByteArray(32)
        for (i in 0 until 32) {
            val hi = Character.digit(trimmed[i * 2], 16)
            val lo = Character.digit(trimmed[i * 2 + 1], 16)
            if (hi < 0 || lo < 0) return null
            out[i] = ((hi shl 4) or lo).toByte()
        }
        return out
    }

    fun encode(bytes: ByteArray): String =
        bytes.joinToString(separator = "") { b -> "%02x".format(b.toInt() and 0xFF) }
}

const val MAX_CLIPBOARD_IMAGE_BYTES: Int = 16 * 1024 * 1024

private val BUILT_IN_IGNORED_CLIPBOARD_MIMES = setOf(
    "x-kde-passwordmanagerhint",
    "application/x-password",
)

fun normalizeClipboardMime(raw: String): String =
    raw.substringBefore(';').trim().lowercase()

fun shouldIgnoreClipboardMimes(
    offeredMimes: List<String>,
    ignoredMimes: List<String>,
): Boolean {
    val rules = buildSet {
        addAll(BUILT_IN_IGNORED_CLIPBOARD_MIMES)
        for (rule in ignoredMimes) {
            val clean = normalizeClipboardMime(rule)
            if (clean.isNotEmpty()) add(clean)
        }
    }
    return offeredMimes.any { mime ->
        val norm = normalizeClipboardMime(mime)
        norm.isNotEmpty() && norm in rules
    }
}

fun shouldSkipClipDescription(
    offeredMimes: List<String>,
    isSensitive: Boolean,
    ignoredMimes: List<String>,
): Boolean = isSensitive || shouldIgnoreClipboardMimes(offeredMimes, ignoredMimes)

fun shouldReadClipboardOnFocus(hasFocus: Boolean, alreadyHandled: Boolean): Boolean =
    hasFocus && !alreadyHandled

fun usePendingIntentTileCollapse(sdkInt: Int): Boolean =
    sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE

object CoreClientProvider {
    @Volatile
    private var instance: UniffiCoreClient? = null

    fun init(context: Context): UniffiCoreClient {
        val client = getInternal()
        client.attachContext(context.applicationContext)
        return client
    }

    fun get(): CoreBridge = getInternal()

    fun getClient(): UniffiCoreClient = getInternal()

    private fun getInternal(): UniffiCoreClient {
        val current = instance
        if (current != null) return current
        return synchronized(this) {
            instance ?: UniffiCoreClient().also { instance = it }
        }
    }
}
