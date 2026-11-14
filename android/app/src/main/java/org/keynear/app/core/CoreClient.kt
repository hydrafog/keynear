package org.keynear.app.core

import android.Manifest
import android.companion.CompanionDeviceManager
import android.content.ContentValues
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.os.PowerManager
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.util.Log
import androidx.core.content.ContextCompat
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.lan.LocalNetworkConstants
import org.keynear.app.service.KeynearTransferJobService
import org.keynear.app.share.DirectShareManager
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiDaemonConfig
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.FfiTransferState
import uniffi.keynear_core.FfiVisibilityMode
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.io.FileOutputStream
import java.security.SecureRandom
import java.util.Locale
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.roundToInt

enum class VisibilityState {
    HIDDEN,
    PAIRED_ONLY,
    EVERYONE_NEARBY;

    fun next(): VisibilityState = when (this) {
        HIDDEN -> PAIRED_ONLY
        PAIRED_ONLY -> EVERYONE_NEARBY
        EVERYONE_NEARBY -> HIDDEN
    }

    fun label(): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> "Everyone nearby"
    }

    fun formattedStatus(remainingSecs: Long): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> {
            val clamped = remainingSecs.coerceAtLeast(0L)
            val mins = clamped / 60L
            val secs = clamped % 60L
            "Everyone (%02d:%02d)".format(mins, secs)
        }
    }

    fun toFfi(): FfiVisibilityMode = when (this) {
        HIDDEN -> FfiVisibilityMode.HIDDEN
        PAIRED_ONLY -> FfiVisibilityMode.PAIRED_ONLY
        EVERYONE_NEARBY -> FfiVisibilityMode.EVERYONE_NEARBY
    }

    companion object {
        fun fromFfi(mode: FfiVisibilityMode): VisibilityState = when (mode) {
            FfiVisibilityMode.HIDDEN -> HIDDEN
            FfiVisibilityMode.PAIRED_ONLY -> PAIRED_ONLY
            FfiVisibilityMode.EVERYONE_NEARBY -> EVERYONE_NEARBY
        }
    }
}

enum class BluetoothStatus {
    READY,
    MISSING_PERMISSION,
    ADAPTER_DISABLED,
    UNAVAILABLE,
}

enum class LocalNetworkStatus {
    READY,
    MISSING_PERMISSION,
    BLOCKED_LNP,
    UNAVAILABLE,
}

enum class DeviceRoleUi {
    DESKTOP,
    MOBILE;

    fun label(): String = when (this) {
        DESKTOP -> "Desktop"
        MOBILE -> "Mobile"
    }

    companion object {
        fun fromFfi(role: FfiDeviceRole): DeviceRoleUi = when (role) {
            FfiDeviceRole.DESKTOP -> DESKTOP
            FfiDeviceRole.MOBILE -> MOBILE
        }
    }
}

enum class TransferDirectionUi {
    SEND,
    RECEIVE;

    companion object {
        fun fromFfi(dir: FfiTransferDirection): TransferDirectionUi = when (dir) {
            FfiTransferDirection.SEND -> SEND
            FfiTransferDirection.RECEIVE -> RECEIVE
        }
    }
}

enum class TransferStateUi {
    PENDING_CONSENT,
    ACTIVE,
    COMPLETED,
    CANCELLED,
    FAILED;

    fun isTerminal(): Boolean = this == COMPLETED || this == CANCELLED || this == FAILED

    fun label(): String = when (this) {
        PENDING_CONSENT -> "Awaiting consent"
        ACTIVE -> "Transferring"
        COMPLETED -> "Completed"
        CANCELLED -> "Cancelled"
        FAILED -> "Failed"
    }

    companion object {
        fun fromFfi(state: FfiTransferState): TransferStateUi = when (state) {
            FfiTransferState.PENDING_CONSENT -> PENDING_CONSENT
            FfiTransferState.ACTIVE -> ACTIVE
            FfiTransferState.COMPLETED -> COMPLETED
            FfiTransferState.CANCELLED -> CANCELLED
            FfiTransferState.FAILED -> FAILED
        }
    }
}

data class TransferUi(
    val transferId: String,
    val peerDeviceId: String,
    val peerDisplayName: String,
    val summaryName: String,
    val mimeType: String,
    val isDirectory: Boolean,
    val direction: TransferDirectionUi,
    val totalBytes: Long,
    val transferredBytes: Long,
    val state: TransferStateUi,
    val startedAt: String,
)

data class DiscoveredPeerUi(
    val address: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val rssi: Int,
    val capabilityFlags: Long,
    val isPaired: Boolean,
    val batteryLevel: Int? = null,
)

data class OrbitPlacement(
    val radiusPx: Int,
    val durationSec: Int,
    val phaseSec: Float,
)

fun orbitFor(rssi: Int, index: Int, count: Int): OrbitPlacement {
    val clamped = rssi.coerceIn(-90, -40)
    val strength = ((-clamped - 40f) / 50f).coerceIn(0f, 1f)
    val radius = (128f + strength * 48f).roundToInt()
    val safeCount = maxOf(1, count)
    return OrbitPlacement(
        radiusPx = radius,
        durationSec = 40,
        phaseSec = (index.toFloat() / safeCount.toFloat()) * 40f,
    )
}

enum class SendItemKind {
    FILE,
    FOLDER,
    CLIPBOARD,
    TEXT,
}

data class SendQueueItemUi(
    val id: String,
    val name: String,
    val sizeBytes: Long,
    val kind: SendItemKind,
    val mime: String,
    val uriString: String = "",
    val textPayload: String = "",
)

fun formatBytes(bytes: Long): String {
    if (bytes <= 0L) return "0 B"
    if (bytes < 1024L) return "$bytes B"
    val kb = bytes.toDouble() / 1024.0
    if (kb < 1024.0) return String.format(Locale.US, "%.1f KB", kb)
    val mb = kb / 1024.0
    if (mb < 1024.0) return String.format(Locale.US, "%.1f MB", mb)
    val gb = mb / 1024.0
    return String.format(Locale.US, "%.1f GB", gb)
}

data class DaemonConfigUi(
    val incomingDirectory: String = "~/Downloads/Keynear",
    val autoAcceptFromPaired: Boolean = false,
    val clipboardSyncEnabled: Boolean = true,
    val clipboardImagesEnabled: Boolean = true,
    val otpPassthroughEnabled: Boolean = true,
    val audioHandoffEnabled: Boolean = false,
    val ignoredMimeTypes: List<String> = DEFAULT_IGNORED_MIMES,
    val notifyAutoAcceptComplete: Boolean = false,
    val darkTheme: Boolean = false,
) {
    fun toFfi(visibility: VisibilityState): FfiDaemonConfig = FfiDaemonConfig(
        visibilityMode = visibility.toFfi(),
        incomingDirectory = incomingDirectory,
        autoAcceptFromPaired = autoAcceptFromPaired,
        clipboardSyncEnabled = clipboardSyncEnabled,
        clipboardImagesEnabled = clipboardImagesEnabled,
        otpPassthroughEnabled = otpPassthroughEnabled,
        audioHandoffEnabled = audioHandoffEnabled,
        ignoredMimeTypes = ignoredMimeTypes,
        notifyAutoAcceptComplete = notifyAutoAcceptComplete,
    )

    companion object {
        val DEFAULT_IGNORED_MIMES = listOf(
            "x-kde-passwordManagerHint",
            "application/x-password",
        )

        fun fromFfi(ffi: FfiDaemonConfig, darkTheme: Boolean): DaemonConfigUi = DaemonConfigUi(
            incomingDirectory = ffi.incomingDirectory,
            autoAcceptFromPaired = ffi.autoAcceptFromPaired,
            clipboardSyncEnabled = ffi.clipboardSyncEnabled,
            clipboardImagesEnabled = ffi.clipboardImagesEnabled,
            otpPassthroughEnabled = ffi.otpPassthroughEnabled,
            audioHandoffEnabled = ffi.audioHandoffEnabled,
            ignoredMimeTypes = ffi.ignoredMimeTypes,
            notifyAutoAcceptComplete = ffi.notifyAutoAcceptComplete,
            darkTheme = darkTheme,
        )
    }
}

data class PairedDeviceUi(
    val deviceId: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val publicKeyFingerprint: String,
    val lastBleAddress: String,
    val pairedAt: String,
    val autoAcceptFiles: Boolean = false,
    val lastEndpointId: String = "",
    val lastLanAddress: String = "",
    val isOnline: Boolean = lastLanAddress.isNotBlank(),
    val rssi: Int = if (lastLanAddress.isNotBlank()) -52 else -76,
)

data class PendingPairingUi(
    val peerAddress: String,
    val peerName: String,
    val verificationCode: String,
    val remainingSecs: Long,
)

data class CoreSnapshot(
    val version: String,
    val maxControlFrameBytes: Long,
    val ready: Boolean,
    val visibility: VisibilityState,
    val configSummary: String,
    val everyoneRemainingSecs: Long = 0L,
    val pairedDevices: List<PairedDeviceUi> = emptyList(),
    val nearbyPeers: List<DiscoveredPeerUi> = emptyList(),
    val pendingPairing: PendingPairingUi? = null,
    val bluetoothStatus: BluetoothStatus = BluetoothStatus.READY,
    val warningMessage: String? = null,
    val localNetworkStatus: LocalNetworkStatus = LocalNetworkStatus.READY,
    val localNetworkMessage: String? = null,
    val activeTransfers: List<TransferUi> = emptyList(),
    val daemonConfig: DaemonConfigUi = DaemonConfigUi(),
    val sendQueue: List<SendQueueItemUi> = emptyList(),
)

sealed interface CoreState {
    data object Loading : CoreState
    data class Empty(val snapshot: CoreSnapshot) : CoreState
    data class Ready(val snapshot: CoreSnapshot) : CoreState
    data class Error(val message: String, val previous: CoreSnapshot? = null) : CoreState
}

sealed interface ShareValidationResult {
    data class Valid(val itemCount: Int, val hasText: Boolean) : ShareValidationResult
    data class Invalid(val reason: String) : ShareValidationResult
}

enum class TransferProgressStage {
    PREPARE,
    TRANSFER,
    COMPLETE,
}

data class TransferProgressModel(
    val stage: TransferProgressStage,
    val overallPercent: Int,
    val segmentedProgress: Int,
    val usesProgressStyleApi36: Boolean,
    val segmentWeights: List<Int> = TRANSFER_SEGMENT_WEIGHTS,
)

private val TRANSFER_SEGMENT_WEIGHTS = listOf(20, 65, 15)

fun computeTransferProgressModel(
    state: TransferStateUi,
    transferredBytes: Long,
    totalBytes: Long,
    sdkInt: Int = Build.VERSION.SDK_INT,
): TransferProgressModel {
    val usesApi36 = sdkInt >= 36
    return when (state) {
        TransferStateUi.COMPLETED -> TransferProgressModel(
            stage = TransferProgressStage.COMPLETE,
            overallPercent = 100,
            segmentedProgress = 100,
            usesProgressStyleApi36 = usesApi36,
        )
        TransferStateUi.PENDING_CONSENT -> TransferProgressModel(
            stage = TransferProgressStage.PREPARE,
            overallPercent = 10,
            segmentedProgress = 10,
            usesProgressStyleApi36 = usesApi36,
        )
        TransferStateUi.ACTIVE -> {
            if (transferredBytes <= 0L) {
                TransferProgressModel(
                    stage = TransferProgressStage.PREPARE,
                    overallPercent = 15,
                    segmentedProgress = 15,
                    usesProgressStyleApi36 = usesApi36,
                )
            } else if (totalBytes > 0L) {
                val ratio = (transferredBytes.toDouble() / totalBytes.toDouble()).coerceIn(0.0, 1.0)
                TransferProgressModel(
                    stage = TransferProgressStage.TRANSFER,
                    overallPercent = (ratio * 100.0).toInt().coerceIn(0, 100),
                    segmentedProgress = 20 + (ratio * 65.0).toInt().coerceIn(0, 65),
                    usesProgressStyleApi36 = usesApi36,
                )
            } else {
                TransferProgressModel(
                    stage = TransferProgressStage.TRANSFER,
                    overallPercent = 50,
                    segmentedProgress = 52,
                    usesProgressStyleApi36 = usesApi36,
                )
            }
        }
        TransferStateUi.CANCELLED, TransferStateUi.FAILED -> TransferProgressModel(
            stage = TransferProgressStage.PREPARE,
            overallPercent = 0,
            segmentedProgress = 0,
            usesProgressStyleApi36 = usesApi36,
        )
    }
}

const val MAX_DISPLAY_NAME_LEN = 64

fun validateDisplayNameInput(raw: String): Result<String> {
    val trimmed = raw.trim()
    if (trimmed.isEmpty()) {
        return Result.failure(IllegalArgumentException("Device name cannot be empty"))
    }
    if (trimmed.length > MAX_DISPLAY_NAME_LEN) {
        return Result.failure(IllegalArgumentException("Device name must be $MAX_DISPLAY_NAME_LEN characters or fewer"))
    }
    if (trimmed.any { it.isISOControl() }) {
        return Result.failure(IllegalArgumentException("Device name contains unsupported control characters"))
    }
    return Result.success(trimmed)
}

data class PermissionsChecklistState(
    val nearbyGranted: Boolean,
    val notificationsGranted: Boolean,
    val batteryOptimizationIgnored: Boolean,
    val companionAssociated: Boolean,
    val localNetworkGranted: Boolean = true,
) {
    val allRequiredGranted: Boolean
        get() = nearbyGranted && localNetworkGranted && notificationsGranted

    val allRecommendedComplete: Boolean
        get() = allRequiredGranted && batteryOptimizationIgnored

    val completedCount: Int
        get() = listOf(
            nearbyGranted,
            notificationsGranted,
            batteryOptimizationIgnored,
            companionAssociated,
        ).count { it }

    val missingPermissionsCount: Int
        get() = listOf(
            nearbyGranted,
            localNetworkGranted,
            companionAssociated,
            notificationsGranted,
            batteryOptimizationIgnored,
        ).count { !it }
}

fun evaluatePermissionsChecklist(
    context: Context,
    sdkInt: Int = Build.VERSION.SDK_INT,
): PermissionsChecklistState {
    val bleReady = KeynearBleConstants.hasScanPermission(context) &&
        KeynearBleConstants.hasConnectPermission(context) &&
        KeynearBleConstants.hasAdvertisePermission(context)
    val wifiNearbyReady = sdkInt < 33 ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.NEARBY_WIFI_DEVICES) ==
        PackageManager.PERMISSION_GRANTED
    val lnpReady = LocalNetworkConstants.hasLocalNetworkPermission(context, sdkInt)
    val notificationsReady = sdkInt < 33 ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
    val batteryIgnored = runCatching {
        context.getSystemService(PowerManager::class.java)
            ?.isIgnoringBatteryOptimizations(context.packageName) == true
    }.getOrDefault(false)
    val companionAssociated = runCatching {
        val manager = context.getSystemService(CompanionDeviceManager::class.java) ?: return@runCatching false
        if (sdkInt >= 33) {
            manager.myAssociations.isNotEmpty()
        } else {
            @Suppress("DEPRECATION")
            manager.associations.isNotEmpty()
        }
    }.getOrDefault(false)

    return PermissionsChecklistState(
        nearbyGranted = bleReady && wifiNearbyReady,
        notificationsGranted = notificationsReady,
        batteryOptimizationIgnored = batteryIgnored,
        companionAssociated = companionAssociated,
        localNetworkGranted = lnpReady,
    )
}

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

private class AndroidMediaStoreOps(
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

class UniffiCoreClient(
    private var appContext: Context? = null,
) : CoreBridge {
    private val handle: CoreHandle by lazy { CoreHandle() }
    private var devicesPath: String? = appContext?.let { File(it.filesDir, "devices.toml").absolutePath }
    private var configPath: String? = appContext?.let { File(it.filesDir, "config.toml").absolutePath }
    private var secretStore: KeystoreSecretStore? = appContext?.let { runCatching { KeystoreSecretStore.open(it) }.getOrNull() }
    private var localDeviceId: String = appContext?.let { resolveLocalDeviceId(it) } ?: "android-device"
    private var mediaStoreTracker: PendingMediaStoreTracker<Uri>? =
        appContext?.let { PendingMediaStoreTracker(AndroidMediaStoreOps(it.applicationContext)) }
    private val sendQueueStore = SendQueueStore()

    @Volatile
    private var cachedDarkTheme: Boolean = appContext?.let { readDarkThemePref(it) } ?: false

    @Volatile
    private var cachedDaemonConfig: DaemonConfigUi = DaemonConfigUi(darkTheme = cachedDarkTheme)

    @Volatile
    private var orionNode: OrionNodeHandle? = null

    @Volatile
    private var cachedNearby: List<DiscoveredPeerUi> = emptyList()

    @Volatile
    private var cachedPendingPairing: PendingPairingUi? = null

    @Volatile
    private var cachedBtStatus: BluetoothStatus = BluetoothStatus.READY

    @Volatile
    private var cachedWarning: String? = null

    @Volatile
    private var cachedLnpStatus: LocalNetworkStatus = LocalNetworkStatus.READY

    @Volatile
    private var cachedLnpMessage: String? = null

    private val notifiedConsentIds = ConcurrentHashMap.newKeySet<String>()
    private val notifiedTerminalIds = ConcurrentHashMap.newKeySet<String>()
    private val fdHandedOffIds = ConcurrentHashMap.newKeySet<String>()

    @Volatile
    private var lastSyncedShortcutSnapshot: List<Pair<String, String>>? = null

    @Synchronized
    fun attachContext(context: Context) {
        val app = context.applicationContext
        if (appContext == null) {
            appContext = app
        }
        if (devicesPath == null) {
            devicesPath = File(app.filesDir, "devices.toml").absolutePath
        }
        if (configPath == null) {
            configPath = File(app.filesDir, "config.toml").absolutePath
        }
        if (secretStore == null) {
            secretStore = runCatching { KeystoreSecretStore.open(app) }.getOrNull()
        }
        if (mediaStoreTracker == null) {
            mediaStoreTracker = PendingMediaStoreTracker(AndroidMediaStoreOps(app))
        }
        localDeviceId = resolveLocalDeviceId(app)
        cachedDarkTheme = readDarkThemePref(app)
        refreshConfigFromDisk()
    }

    private fun readDarkThemePref(context: Context): Boolean =
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .getBoolean(PREF_DARK_THEME, false)

    private fun writeDarkThemePref(context: Context, enabled: Boolean) {
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(PREF_DARK_THEME, enabled)
            .apply()
    }

    private fun refreshConfigFromDisk() {
        val path = configPath ?: return
        runCatching {
            val loaded = handle.loadDaemonConfig(path)
            handle.setVisibilityMode(loaded.visibilityMode)
            cachedDaemonConfig = DaemonConfigUi.fromFfi(loaded, cachedDarkTheme)
        }
    }

    fun coreHandle(): CoreHandle = handle

    fun keystoreStore(): KeystoreSecretStore? = secretStore

    fun devicesTomlPath(): String? = devicesPath

    fun localDeviceId(): String = localDeviceId

    fun localDisplayName(): String = android.os.Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android"

    fun getOrCreateStaticPrivateKey(): ByteArray {
        val store = secretStore
        return if (store != null) {
            store.getOrCreateStaticPrivateKey { handle.generateStaticKeypair().privateKey }
        } else {
            handle.generateStaticKeypair().privateKey
        }
    }

    private fun getOrCreateQuicSecretKey(): ByteArray {
        val randomGen = {
            ByteArray(32).also { SecureRandom().nextBytes(it) }
        }
        val store = secretStore
        return if (store != null) {
            store.getOrCreateQuicSecretKey(randomGen)
        } else {
            randomGen()
        }
    }

    @Synchronized
    fun ensureOrionNode(context: Context): OrionNodeHandle? {
        attachContext(context)
        orionNode?.let { return it }
        val app = context.applicationContext
        val incomingDir = File(app.filesDir, "incoming").apply { mkdirs() }
        val quicKey = getOrCreateQuicSecretKey()
        return try {
            val node = OrionNodeHandle(
                deviceId = localDeviceId,
                displayName = localDisplayName(),
                capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
                quicSecretKey = quicKey,
                incomingDirectory = incomingDir.absolutePath,
                autoAcceptFromPaired = false,
                requireExplicitFdForSingleFile = true,
            )
            orionNode = node
            syncAllPairedPeersToOrion(node)
            node
        } catch (t: Throwable) {
            Log.w(TAG, "orion_node_start_failed: ${t.message}")
            null
        } finally {
            quicKey.fill(0)
        }
    }

    @Synchronized
    fun shutdownOrion() {
        mediaStoreTracker?.cleanupAllPending()
        orionNode?.let { node ->
            runCatching { node.shutdown() }
            runCatching { node.close() }
        }
        orionNode = null
        fdHandedOffIds.clear()
        notifiedConsentIds.clear()
        notifiedTerminalIds.clear()
    }

    fun localLanEndpoint(): FfiLanEndpoint? = orionNode?.localLanEndpoint()

    fun syncAllPairedPeersToOrion(node: OrionNodeHandle? = orionNode) {
        val activeNode = node ?: return
        val store = secretStore ?: return
        for (dev in getPairedDevices()) {
            val eidBytes = decodeEndpointIdHex(dev.lastEndpointId) ?: continue
            val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
            try {
                val addrs = if (dev.lastLanAddress.isBlank()) emptyList() else listOf(dev.lastLanAddress)
                runCatching {
                    activeNode.syncPeer(
                        deviceId = dev.deviceId,
                        endpointId = eidBytes,
                        socketAddrs = addrs,
                        reconnectSecret = secrets.reconnectSecret,
                        autoAcceptFiles = dev.autoAcceptFiles,
                    )
                }
            } finally {
                secrets.zeroize()
            }
        }
    }

    fun encodeLocalLanEndpointForBleAddress(bleAddress: String): ByteArray? {
        val endpoint = orionNode?.localLanEndpoint() ?: return null
        val store = secretStore ?: return null
        val devices = getPairedDevices()
        val matched = devices.firstOrNull { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            ?: if (devices.size == 1) devices.firstOrNull() else null
        matched ?: return null
        val secrets = store.getPeerSecrets(matched.deviceId) ?: return null
        return try {
            handle.encodeLanEndpoint(endpoint, secrets.reconnectSecret)
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
    }

    fun encodeLocalLanEndpointForPeer(deviceId: String): ByteArray? {
        val endpoint = orionNode?.localLanEndpoint() ?: return null
        val store = secretStore ?: return null
        val secrets = store.getPeerSecrets(deviceId) ?: return null
        return try {
            handle.encodeLanEndpoint(endpoint, secrets.reconnectSecret)
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
    }

    fun onEncryptedLanEndpointFromBle(encoded: ByteArray, bleAddress: String? = null): FfiLanEndpoint? {
        if (encoded.isEmpty() || encoded.size > 65_536) return null
        val store = secretStore ?: return null
        val devices = getPairedDevices()
        val ordered = if (!bleAddress.isNullOrBlank()) {
            val exact = devices.filter { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            val rest = devices.filterNot { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            exact + rest
        } else {
            devices
        }
        for (dev in ordered) {
            val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
            val decoded = try {
                handle.decodeLanEndpoint(encoded, dev.deviceId, secrets.reconnectSecret)
            } catch (_: Throwable) {
                null
            } finally {
                secrets.zeroize()
            }
            if (decoded != null) {
                onPeerLanEndpointDiscovered(
                    deviceId = decoded.deviceId,
                    endpointId = decoded.endpointId,
                    socketAddrs = decoded.socketAddrs,
                )
                return decoded
            }
        }
        return null
    }

    fun onPeerLanEndpointDiscovered(
        deviceId: String,
        endpointId: ByteArray,
        socketAddrs: List<String>,
    ) {
        if (endpointId.size != 32) return
        val devices = getPairedDevices()
        val dev = devices.firstOrNull { it.deviceId == deviceId } ?: return
        val store = secretStore ?: return
        val secrets = store.getPeerSecrets(deviceId) ?: return
        try {
            orionNode?.syncPeer(
                deviceId = deviceId,
                endpointId = endpointId,
                socketAddrs = socketAddrs,
                reconnectSecret = secrets.reconnectSecret,
                autoAcceptFiles = dev.autoAcceptFiles,
            )
            val path = devicesPath
            val eidHex = encodeHex(endpointId)
            val firstAddr = socketAddrs.firstOrNull().orEmpty()
            if (path != null && (dev.lastEndpointId != eidHex || (firstAddr.isNotEmpty() && dev.lastLanAddress != firstAddr))) {
                runCatching {
                    handle.savePairedDevice(
                        devicesTomlPath = path,
                        device = dev.copy(
                            lastEndpointId = eidHex,
                            lastLanAddress = if (firstAddr.isNotEmpty()) firstAddr else dev.lastLanAddress,
                        ),
                    )
                }
            }
        } finally {
            secrets.zeroize()
        }
    }

    fun syncTransferStates(context: Context): Boolean {
        val app = context.applicationContext
        attachContext(app)
        val node = orionNode ?: return false
        val sessions = runCatching { node.listTransfers() }.getOrNull() ?: return false
        val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
        var changed = false
        var hasActiveTransfers = false

        for (session in sessions) {
            val uiState = TransferStateUi.fromFfi(session.state)
            val peerName = peerNames[session.peerDeviceId] ?: session.peerDeviceId
            val summary = summarizeTransferItems(session)

            if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.PENDING_CONSENT) {
                if (notifiedConsentIds.add(session.transferId)) {
                    PairingNotificationManager.showIncomingTransferConsent(
                        context = app,
                        transferId = session.transferId,
                        peerName = peerName,
                        summary = summary,
                        sizeLabel = formatByteCount(session.totalBytes.toLong()),
                    )
                    changed = true
                }
            }

            if (uiState == TransferStateUi.ACTIVE) {
                hasActiveTransfers = true
                if (session.direction == FfiTransferDirection.RECEIVE && isSingleFileReceive(session) && !fdHandedOffIds.contains(session.transferId)) {
                    if (prepareAndHandOffReceiveFd(node, session)) {
                        changed = true
                    }
                }
                PairingNotificationManager.showOrUpdateTransferProgress(
                    context = app,
                    transfer = toTransferUi(session, peerNames),
                )
            }

            mediaStoreTracker?.reconcileState(session.transferId, uiState)

            if (uiState.isTerminal()) {
                PairingNotificationManager.cancelTransferNotification(app, session.transferId)
                if (notifiedTerminalIds.add(session.transferId)) {
                    changed = true
                    if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.COMPLETED) {
                        val autoAccepted = getPairedDevices()
                            .firstOrNull { it.deviceId == session.peerDeviceId }
                            ?.autoAcceptFiles == true
                        PairingNotificationManager.showTransferCompleted(
                            context = app,
                            peerName = peerName,
                            summary = summary,
                            autoAccepted = autoAccepted,
                        )
                    } else if (uiState == TransferStateUi.FAILED) {
                        PairingNotificationManager.showTransferError(
                            context = app,
                            summary = summary,
                            reason = "Connection interrupted or rejected",
                        )
                    }
                }
            }
        }
        KeynearTransferJobService.syncActiveTransfers(app, hasActiveTransfers)
        return changed
    }

    private fun isSingleFileReceive(session: FfiTransferSession): Boolean =
        session.items.size == 1 && !session.items[0].isDirectory

    private fun prepareAndHandOffReceiveFd(
        node: OrionNodeHandle,
        session: FfiTransferSession,
    ): Boolean {
        val tracker = mediaStoreTracker ?: return false
        val item = session.items.firstOrNull() ?: return false
        if (!fdHandedOffIds.add(session.transferId)) {
            return false
        }
        val ok = runCatching {
            tracker.prepareAndHandOff(
                transferId = session.transferId,
                fileName = item.name,
                mimeType = item.mimeType,
            ) { rawFd ->
                node.provideReceiveFd(session.transferId, rawFd)
            }
        }.getOrDefault(false)
        if (!ok) {
            fdHandedOffIds.remove(session.transferId)
        }
        return ok
    }

    fun saveCompletedPairing(
        peerDeviceId: String,
        peerDisplayName: String,
        peerRole: FfiDeviceRole,
        peerStaticPubkeyHex: String,
        peerCapabilityFlags: UInt,
        bleAddress: String,
        reconnectSecret: ByteArray,
    ) {
        val store = secretStore
        val path = devicesPath
        if (store != null) {
            store.putPeerSecrets(
                deviceId = peerDeviceId,
                record = PeerSecretRecord(
                    irk = reconnectSecret.copyOf(),
                    reconnectSecret = reconnectSecret.copyOf(),
                ),
            )
        }
        if (path != null) {
            try {
                handle.savePairedDevice(
                    devicesTomlPath = path,
                    device = FfiPairedDevice(
                        deviceId = peerDeviceId,
                        displayName = peerDisplayName,
                        deviceRole = peerRole,
                        staticX25519Pubkey = peerStaticPubkeyHex,
                        lastBleAddress = bleAddress,
                        lastLanAddress = "",
                        lastEndpointId = "",
                        capabilityFlags = peerCapabilityFlags,
                        autoAcceptFiles = false,
                        pairedAt = (System.currentTimeMillis() / 1000L).toString(),
                    ),
                )
            } catch (t: Throwable) {
                store?.deletePeerSecrets(peerDeviceId)
                throw t
            }
        }
    }

    override fun updateNearbyPeers(peers: List<DiscoveredPeerUi>) {
        cachedNearby = peers
    }

    override fun updateBluetoothStatus(status: BluetoothStatus, warning: String?) {
        cachedBtStatus = status
        cachedWarning = warning
    }

    override fun updateLocalNetworkStatus(status: LocalNetworkStatus, message: String?) {
        cachedLnpStatus = status
        cachedLnpMessage = message
    }

    override fun currentLocalNetworkStatus(): LocalNetworkStatus = cachedLnpStatus

    override fun setPendingPairing(prompt: PendingPairingUi?) {
        cachedPendingPairing = prompt
    }

    override fun loadState(): CoreState = buildState()

    override fun cycleVisibility(): CoreState {
        return try {
            handle.cycleVisibilityMode()
            val mode = VisibilityState.fromFfi(handle.visibilityMode())
            persistCurrentConfig(mode)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to cycle visibility mode")
        }
    }

    override fun setVisibility(mode: VisibilityState): CoreState {
        return try {
            handle.setVisibilityMode(mode.toFfi())
            persistCurrentConfig(mode)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to set visibility mode", snapshotFromCurrent())
        }
    }

    private fun persistCurrentConfig(visibility: VisibilityState) {
        val path = configPath ?: return
        runCatching {
            val saved = handle.saveDaemonConfig(path, cachedDaemonConfig.toFfi(visibility))
            cachedDaemonConfig = DaemonConfigUi.fromFfi(saved, cachedDarkTheme)
        }
    }

    override fun resetBleBond(deviceId: String): CoreState {
        return try {
            secretStore?.deletePeerSecrets(deviceId)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to reset Bluetooth bond", snapshotFromCurrent())
        }
    }

    override fun stageFileUris(uris: List<String>): CoreState {
        val ctx = appContext
        uris.forEach { raw ->
            if (raw.isBlank()) return@forEach
            if (ctx != null) {
                val (name, size, mime) = queryUriMetadata(ctx, Uri.parse(raw))
                sendQueueStore.addFile(name, size, mime, raw)
            } else {
                val name = raw.substringAfterLast('/').ifBlank { "shared-file.bin" }
                sendQueueStore.addFile(name, 0L, "application/octet-stream", raw)
            }
        }
        return buildState()
    }

    override fun stageFolderUri(
        folderUri: String,
        displayName: String,
        fileUris: List<String>,
    ): CoreState {
        val ctx = appContext
        if (fileUris.isNotEmpty()) {
            fileUris.forEach { raw ->
                if (raw.isBlank()) return@forEach
                if (ctx != null) {
                    val (name, size, mime) = queryUriMetadata(ctx, Uri.parse(raw))
                    sendQueueStore.addFile(name, size, mime, raw)
                } else {
                    val name = raw.substringAfterLast('/').ifBlank { "shared-file.bin" }
                    sendQueueStore.addFile(name, 0L, "application/octet-stream", raw)
                }
            }
        } else if (folderUri.isNotBlank()) {
            sendQueueStore.addFolder(displayName, folderUri)
        }
        return buildState()
    }

    override fun stageClipboardText(text: String): CoreState {
        sendQueueStore.addClipboard(text)
        return buildState()
    }

    override fun stageTextSnippet(text: String): CoreState {
        sendQueueStore.addTextSnippet(text)
        return buildState()
    }

    override fun removeSendQueueItem(itemId: String): CoreState {
        sendQueueStore.remove(itemId)
        return buildState()
    }

    override fun clearSendQueue(): CoreState {
        sendQueueStore.clear()
        return buildState()
    }

    override fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> {
        val items = sendQueueStore.snapshot()
        if (items.isEmpty()) {
            return Result.failure(IllegalArgumentException("Stage at least one file or snippet first"))
        }
        val uris = items.mapNotNull { item ->
            item.uriString.takeIf { it.isNotBlank() && item.kind != SendItemKind.FOLDER }
        }
        val textCombined = items
            .mapNotNull { it.textPayload.takeIf { txt -> txt.isNotEmpty() } }
            .joinToString(separator = "\n\n")
            .ifBlank { null }
        val result = sendContentUris(peerDeviceId, uris, textCombined)
        if (result.isSuccess) {
            sendQueueStore.clear()
        }
        return result
    }

    override fun updateDaemonConfig(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState {
        return try {
            val updated = transform(cachedDaemonConfig)
            cachedDarkTheme = updated.darkTheme
            appContext?.let { writeDarkThemePref(it, updated.darkTheme) }
            val currentVisibility = VisibilityState.fromFfi(handle.visibilityMode())
            val path = configPath
            cachedDaemonConfig = if (path != null) {
                val saved = handle.saveDaemonConfig(path, updated.toFfi(currentVisibility))
                DaemonConfigUi.fromFfi(saved, updated.darkTheme)
            } else {
                updated
            }
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to save settings", snapshotFromCurrent())
        }
    }

    override fun tick(): CoreState = buildState()

    override fun unpairDevice(deviceId: String): CoreState {
        return try {
            devicesPath?.let { handle.removePairedDevice(it, deviceId) }
            secretStore?.deletePeerSecrets(deviceId)
            runCatching { orionNode?.removePeer(deviceId) }
            lastSyncedShortcutSnapshot = null
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to unpair device")
        }
    }

    override fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState {
        val validated = validateDisplayNameInput(newDisplayName).getOrElse { err ->
            return CoreState.Error(err.message ?: "Invalid device name", snapshotFromCurrent())
        }
        val path = devicesPath ?: return CoreState.Error("Device store path unavailable", snapshotFromCurrent())
        return try {
            handle.renamePairedDevice(path, deviceId, validated)
            lastSyncedShortcutSnapshot = null
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to rename device", snapshotFromCurrent())
        }
    }

    override fun setAutoAcceptFiles(deviceId: String, enabled: Boolean): CoreState {
        val path = devicesPath ?: return CoreState.Error("Device store path unavailable")
        return try {
            handle.setAutoAcceptFiles(path, deviceId, enabled)
            syncAllPairedPeersToOrion()
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to update auto-accept preference", snapshotFromCurrent())
        }
    }

    fun acceptIncomingTransferOrSync(context: Context, transferId: String): CoreState {
        attachContext(context)
        return acceptTransfer(transferId)
    }

    override fun acceptTransfer(transferId: String): CoreState {
        val node = orionNode ?: return CoreState.Error("Orion transfer node is not running", snapshotFromCurrent())
        return try {
            val session = node.respondConsent(transferId, true)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            if (isSingleFileReceive(session)) {
                val handedOff = prepareAndHandOffReceiveFd(node, session)
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

    override fun declineTransfer(transferId: String): CoreState {
        val node = orionNode ?: return buildState()
        return try {
            node.respondConsent(transferId, false)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to decline transfer", snapshotFromCurrent())
        }
    }

    override fun cancelTransfer(transferId: String): CoreState {
        val node = orionNode ?: return buildState()
        return try {
            node.cancelTransfer(transferId)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to cancel transfer", snapshotFromCurrent())
        }
    }

    override fun sendContentUris(
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> {
        val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
        val node = ensureOrionNode(ctx)
            ?: return Result.failure(IllegalStateException("Local transfer node could not start"))
        syncAllPairedPeersToOrion(node)

        val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
        return runCatching {
            val session = if (uris.size == 1 && sharedText.isNullOrBlank()) {
                val uri = Uri.parse(uris.first())
                val (fileName, sizeHint, mime) = queryUriMetadata(ctx, uri)
                val pfd = ctx.contentResolver.openFileDescriptor(uri, "r")
                    ?: error("Unable to open shared file descriptor")
                val sizeBytes = if (sizeHint > 0L) sizeHint else pfd.statSize.coerceAtLeast(0L)
                val rawFd = pfd.detachFd()
                node.sendFd(
                    peerDeviceId = peerDeviceId,
                    fileName = fileName,
                    mimeType = mime,
                    sizeBytes = sizeBytes.toULong(),
                    rawFd = rawFd,
                )
            } else {
                val stagingDir = File(ctx.cacheDir, "orion_share_${UUID.randomUUID()}").apply { mkdirs() }
                try {
                    val stagedPaths = mutableListOf<String>()
                    uris.forEachIndexed { idx, raw ->
                        val uri = Uri.parse(raw)
                        val (name, _, _) = queryUriMetadata(ctx, uri)
                        val safeName = "${idx}_${name.replace('/', '_')}"
                        val dest = File(stagingDir, safeName)
                        ctx.contentResolver.openInputStream(uri)?.use { input ->
                            FileOutputStream(dest).use { output ->
                                input.copyTo(output, bufferSize = 65_536)
                            }
                        } ?: error("Unable to read shared item $name")
                        stagedPaths.add(dest.absolutePath)
                    }
                    val trimmedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
                    if (trimmedText != null) {
                        val textFile = File(stagingDir, "shared-text.txt")
                        textFile.writeText(trimmedText, Charsets.UTF_8)
                        stagedPaths.add(textFile.absolutePath)
                    }
                    require(stagedPaths.isNotEmpty()) { "No shareable payload found" }
                    node.sendPaths(peerDeviceId, stagedPaths)
                } finally {
                    stagingDir.deleteRecursively()
                }
            }
            val uiTransfer = toTransferUi(session, peerNames)
            PairingNotificationManager.showOrUpdateTransferProgress(ctx, uiTransfer)
            KeynearTransferJobService.syncActiveTransfers(ctx, true)
            uiTransfer
        }
    }

    private fun queryUriMetadata(context: Context, uri: Uri): Triple<String, Long, String> {
        var name = uri.lastPathSegment?.substringAfterLast('/')?.ifBlank { null } ?: "shared-file.bin"
        var size = 0L
        runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
                ?.use { cursor ->
                    if (cursor.moveToFirst()) {
                        val nameIdx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        if (nameIdx >= 0) {
                            cursor.getString(nameIdx)?.takeIf { it.isNotBlank() }?.let { name = it }
                        }
                        val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                        if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                            size = cursor.getLong(sizeIdx)
                        }
                    }
                }
        }
        val mime = context.contentResolver.getType(uri)?.ifBlank { null } ?: "application/octet-stream"
        return Triple(name, size, mime)
    }

    fun getPairedDevices(): List<FfiPairedDevice> {
        val path = devicesPath ?: return emptyList()
        return runCatching { handle.loadPairedDevices(path) }.getOrDefault(emptyList())
    }

    fun timeoutSecs(): Long = runCatching { everyoneNearbyTimeoutSecs().toLong() }.getOrDefault(600L)

    private fun snapshotFromCurrent(): CoreSnapshot? = when (val state = runCatching { buildState() }.getOrNull()) {
        is CoreState.Empty -> state.snapshot
        is CoreState.Ready -> state.snapshot
        else -> null
    }

    private fun buildState(): CoreState = try {
        val status = handle.status()
        val visibility = VisibilityState.fromFfi(handle.visibilityMode())
        val remaining = handle.everyoneNearbyRemainingSecs().toLong()
        val rawPaired = getPairedDevices()
        val peerNames = rawPaired.associate { it.deviceId to it.displayName }
        val paired = rawPaired.map { dev ->
            val online = dev.lastLanAddress.isNotBlank()
            PairedDeviceUi(
                deviceId = dev.deviceId,
                displayName = dev.displayName,
                role = DeviceRoleUi.fromFfi(dev.deviceRole),
                publicKeyFingerprint = dev.staticX25519Pubkey.take(12),
                lastBleAddress = dev.lastBleAddress,
                pairedAt = dev.pairedAt,
                autoAcceptFiles = dev.autoAcceptFiles,
                lastEndpointId = dev.lastEndpointId,
                lastLanAddress = dev.lastLanAddress,
                isOnline = online,
                rssi = if (online) -52 else -76,
            )
        }
        val shortcutSnapshot = paired.map { it.deviceId to it.displayName }
        appContext?.let { ctx ->
            if (lastSyncedShortcutSnapshot != shortcutSnapshot) {
                lastSyncedShortcutSnapshot = shortcutSnapshot
                DirectShareManager.syncShortcuts(ctx, paired)
            }
        }
        val transfers = orionNode?.let { node ->
            runCatching { node.listTransfers() }.getOrDefault(emptyList())
                .map { toTransferUi(it, peerNames) }
        } ?: emptyList()

        val pending = cachedPendingPairing?.let { prompt ->
            if (prompt.remainingSecs <= 1L) null else prompt.copy(remainingSecs = prompt.remainingSecs - 1L)
        }
        if (cachedPendingPairing != null) {
            cachedPendingPairing = pending
        }
        val activeConfig = configPath?.let { path ->
            runCatching {
                DaemonConfigUi.fromFfi(handle.loadDaemonConfig(path), cachedDarkTheme)
            }.getOrNull()
        }?.also { cachedDaemonConfig = it } ?: cachedDaemonConfig.copy(darkTheme = cachedDarkTheme)

        val queue = sendQueueStore.snapshot()
        val snapshot = CoreSnapshot(
            version = status.version,
            maxControlFrameBytes = status.maxControlFrameBytes.toLong(),
            ready = status.ready,
            visibility = visibility,
            configSummary = defaultConfigSummary(),
            everyoneRemainingSecs = remaining,
            pairedDevices = paired,
            nearbyPeers = cachedNearby,
            pendingPairing = cachedPendingPairing,
            bluetoothStatus = cachedBtStatus,
            warningMessage = cachedWarning,
            localNetworkStatus = cachedLnpStatus,
            localNetworkMessage = cachedLnpMessage,
            activeTransfers = transfers,
            daemonConfig = activeConfig,
            sendQueue = queue,
        )
        if (paired.isEmpty() && cachedNearby.isEmpty() && cachedPendingPairing == null && transfers.isEmpty() && queue.isEmpty()) {
            CoreState.Empty(snapshot)
        } else {
            CoreState.Ready(snapshot)
        }
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to initialize keynear core")
    }

    private fun toTransferUi(session: FfiTransferSession, peerNames: Map<String, String>): TransferUi =
        TransferUi(
            transferId = session.transferId,
            peerDeviceId = session.peerDeviceId,
            peerDisplayName = peerNames[session.peerDeviceId] ?: session.peerDeviceId,
            summaryName = summarizeTransferItems(session),
            mimeType = session.items.firstOrNull()?.mimeType ?: "application/octet-stream",
            isDirectory = session.items.any { it.isDirectory },
            direction = TransferDirectionUi.fromFfi(session.direction),
            totalBytes = session.totalBytes.toLong().coerceAtLeast(0L),
            transferredBytes = session.transferredBytes.toLong().coerceAtLeast(0L),
            state = TransferStateUi.fromFfi(session.state),
            startedAt = session.startedAt,
        )

    private fun summarizeTransferItems(session: FfiTransferSession): String {
        val items = session.items
        if (items.isEmpty()) return "Transfer"
        if (items.size == 1) return items[0].name
        return "${items[0].name} (+${items.size - 1} more)"
    }

    private fun resolveLocalDeviceId(context: Context): String {
        val prefs = context.getSharedPreferences("keynear_device_identity", Context.MODE_PRIVATE)
        val existing = prefs.getString("device_id", null)
        if (!existing.isNullOrBlank()) {
            return existing
        }
        val generated = "android-${UUID.randomUUID().toString().take(12)}"
        prefs.edit().putString("device_id", generated).apply()
        return generated
    }

    private fun decodeEndpointIdHex(hex: String): ByteArray? = EndpointHexCodec.decode32(hex)

    private fun encodeHex(bytes: ByteArray): String = EndpointHexCodec.encode(bytes)

    companion object {
        private const val TAG = "UniffiCoreClient"
        private const val UI_PREFS_NAME = "keynear_ui_prefs"
        private const val PREF_DARK_THEME = "dark_theme"
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
