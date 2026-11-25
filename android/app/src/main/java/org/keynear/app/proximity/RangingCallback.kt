package org.keynear.app.proximity

import android.ranging.RangingData
import android.ranging.RangingDevice
import android.ranging.RangingSession
import uniffi.keynear_core.FfiProximitySnapshot
import uniffi.keynear_core.FfiProximityStatus
import uniffi.keynear_core.FfiRangingSample
import uniffi.keynear_core.FfiRangingTechnology
import uniffi.keynear_core.evaluateRangingSequence

object RangingDecision {
    const val DEFAULT_ENTER_DISTANCE_METERS: Float = 2.0f
    const val DEFAULT_EXIT_DISTANCE_METERS: Float = 3.5f
    const val DEFAULT_ENTER_RSSI_DBM: Int = -62
    const val DEFAULT_EXIT_RSSI_DBM: Int = -72

    fun inRange(
        distanceMeters: Float?,
        rssiDbm: Int?,
        currentlyInRange: Boolean,
        enterDistanceMeters: Float = DEFAULT_ENTER_DISTANCE_METERS,
        exitDistanceMeters: Float = DEFAULT_EXIT_DISTANCE_METERS,
        enterRssiDbm: Int = DEFAULT_ENTER_RSSI_DBM,
        exitRssiDbm: Int = DEFAULT_EXIT_RSSI_DBM,
    ): Boolean {
        if (distanceMeters != null && distanceMeters >= 0f) {
            val threshold = if (currentlyInRange) exitDistanceMeters else enterDistanceMeters
            return distanceMeters <= threshold
        }
        if (rssiDbm != null) {
            val threshold = if (currentlyInRange) exitRssiDbm else enterRssiDbm
            return rssiDbm >= threshold
        }
        return false
    }

    fun snapshot(samples: List<FfiRangingSample>): FfiProximitySnapshot =
        evaluateRangingSequence(samples)

    fun outOfRange(snapshot: FfiProximitySnapshot): Boolean =
        snapshot.status == FfiProximityStatus.OUT_OF_RANGE
}

class RangingCallback(
    private val deviceId: String,
    private val onSample: (FfiRangingSample) -> Unit,
) : RangingSession.Callback {
    private var sequence = 0L

    override fun onResults(peer: RangingDevice, data: RangingData) {
        sequence += 1
        onSample(
            FfiRangingSample(
                deviceId = deviceId,
                technology = FfiRangingTechnology.UWB,
                distanceMeters = data.distanceMeters.toDouble(),
                rssiDbm = null,
                sequence = sequence,
            )
        )
    }

    override fun onOpened() = Unit
    override fun onOpenFailed(reason: Int) = Unit
    override fun onStarted(peer: RangingDevice, technology: Int) = Unit
    override fun onStopped(peer: RangingDevice, technology: Int) = Unit
    override fun onClosed(reason: Int) = Unit
}
