package org.keynear.app.ui.model

import uniffi.keynear_core.FfiDaemonConfig
import kotlin.math.roundToInt

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
    val dynamicAccent: Boolean = true,
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
            dynamicAccent: Boolean = true,
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
            dynamicAccent = dynamicAccent,
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
