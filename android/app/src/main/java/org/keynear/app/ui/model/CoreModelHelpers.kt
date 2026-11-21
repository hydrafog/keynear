package org.keynear.app.ui.model

import android.os.Build
import java.util.Locale

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
