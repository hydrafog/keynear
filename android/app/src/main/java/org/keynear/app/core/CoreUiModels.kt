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

