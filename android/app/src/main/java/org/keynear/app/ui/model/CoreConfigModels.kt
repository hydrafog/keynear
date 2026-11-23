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
private const val ORBIT_PIN_HALF_HEIGHT_DP = 57f
private const val ORBIT_MIN_RADIUS_DP = 116f
private const val ORBIT_MAX_RADIUS_DP = 172f
private const val ORBIT_MIN_SCALE = 0.62f
private const val ORBIT_INNER_RATIO = 0.8f
private const val ORBIT_RING_COUNT = 3
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

fun orbitPlacementsFor(deviceIds: List<String>, layout: RadarLayout): List<OrbitPlacement> {
    val count = deviceIds.size
    if (count == 0) return emptyList()
    val rankById = deviceIds.indices.sortedBy { deviceIds[it] }
    val ringOf = IntArray(count)
    rankById.forEachIndexed { rank, index ->
        ringOf[index] = when {
            count == 1 -> 1
            count == 2 -> if (rank == 0) 0 else 2
            else -> rank % ORBIT_RING_COUNT
        }
    }
    val ringCounts = IntArray(ORBIT_RING_COUNT)
    ringOf.forEach { ringCounts[it]++ }
    val slotOf = IntArray(count)
    val nextSlot = IntArray(ORBIT_RING_COUNT)
    rankById.forEach { index ->
        val ring = ringOf[index]
        slotOf[index] = nextSlot[ring]++
    }
    val inner = maxOf(ORBIT_MIN_RADIUS_DP, layout.maxRadiusDp * ORBIT_INNER_RATIO)
    return deviceIds.indices.map { index ->
        val ring = ringOf[index]
        val radius = inner + (layout.maxRadiusDp - inner) * ring.toFloat() / (ORBIT_RING_COUNT - 1)
        val phase = (((slotOf[index].toFloat() / ringCounts[ring]) + ring.toFloat() / ORBIT_RING_COUNT) % 1f) * ORBIT_DURATION_SEC
        OrbitPlacement(radius.roundToInt(), ORBIT_DURATION_SEC, phase)
    }
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
