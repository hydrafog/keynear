package org.keynear.app.ui.model

import uniffi.keynear_core.FfiDaemonConfig
import kotlin.math.roundToInt

data class OrbitPlacement(
    val radiusPx: Int,
    val durationSec: Int,
    val phaseSec: Float,
)

data class RadarLayout(
    val scale: Float,
    val maxRadiusDp: Float,
)

private const val ORBIT_DURATION_SEC = 40
private const val ORBIT_PIN_HALF_WIDTH_DP = 82f
private const val ORBIT_PIN_HALF_HEIGHT_DP = 50f
private const val ORBIT_MIN_RADIUS_DP = 116f
private const val ORBIT_MAX_RADIUS_DP = 172f
private const val ORBIT_MIN_SCALE = 0.62f
private const val ORBIT_INNER_RATIO = 0.8f
private const val RSSI_MIN = -90
private const val RSSI_MAX = -40
private const val RSSI_RANGE = 50f

fun radarLayoutFor(widthDp: Float, heightDp: Float): RadarLayout {
    if (widthDp <= 0f || heightDp <= 0f) {
        return RadarLayout(scale = 1f, maxRadiusDp = ORBIT_MIN_RADIUS_DP)
    }
    val fitH = heightDp / (2f * (ORBIT_MIN_RADIUS_DP + ORBIT_PIN_HALF_HEIGHT_DP))
    val fitW = widthDp / (2f * (ORBIT_MIN_RADIUS_DP + ORBIT_PIN_HALF_WIDTH_DP))
    val scale = minOf(fitH, fitW, 1f).coerceIn(ORBIT_MIN_SCALE, 1f)
    val maxRadiusDp = minOf(
        widthDp / (2f * scale) - ORBIT_PIN_HALF_WIDTH_DP,
        heightDp / (2f * scale) - ORBIT_PIN_HALF_HEIGHT_DP,
    ).coerceIn(ORBIT_MIN_RADIUS_DP, ORBIT_MAX_RADIUS_DP)
    return RadarLayout(scale = scale, maxRadiusDp = maxRadiusDp)
}

fun orbitFor(rssi: Int, index: Int, count: Int): OrbitPlacement {
    val clamped = rssi.coerceIn(RSSI_MIN, RSSI_MAX)
    val strength = ((-clamped - 40f) / RSSI_RANGE).coerceIn(0f, 1f)
    val radius = (128f + strength * 48f).roundToInt()
    val safeCount = maxOf(1, count)
    return OrbitPlacement(
        radiusPx = radius,
        durationSec = ORBIT_DURATION_SEC,
        phaseSec = (index.toFloat() / safeCount.toFloat()) * ORBIT_DURATION_SEC.toFloat(),
    )
}

fun orbitFor(rssi: Int, index: Int, count: Int, layout: RadarLayout): OrbitPlacement {
    val clamped = rssi.coerceIn(RSSI_MIN, RSSI_MAX)
    val strength = ((-clamped - 40f) / RSSI_RANGE).coerceIn(0f, 1f)
    val inner = maxOf(ORBIT_MIN_RADIUS_DP, layout.maxRadiusDp * ORBIT_INNER_RATIO)
    val radius = (inner + strength * (layout.maxRadiusDp - inner)).roundToInt()
    val safeCount = maxOf(1, count)
    return OrbitPlacement(
        radiusPx = radius,
        durationSec = ORBIT_DURATION_SEC,
        phaseSec = (index.toFloat() / safeCount.toFloat()) * ORBIT_DURATION_SEC.toFloat(),
    )
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

        fun fromFfi(
            ffi: FfiDaemonConfig,
            darkTheme: Boolean,
        ): DaemonConfigUi = DaemonConfigUi(
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

data class TransferProgressModel(
    val stage: TransferProgressStage,
    val overallPercent: Int,
    val segmentedProgress: Int,
    val usesProgressStyleApi36: Boolean,
    val segmentWeights: List<Int> = TRANSFER_SEGMENT_WEIGHTS,
)

private val TRANSFER_SEGMENT_WEIGHTS = listOf(20, 65, 15)
