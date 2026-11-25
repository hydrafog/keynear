package org.keynear.app.proximity

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.keynear_core.FfiRangingSample
import uniffi.keynear_core.FfiProximityStatus
import uniffi.keynear_core.FfiRangingTechnology

class ProximityDecisionTest {
    @Test
    fun distanceUsesExitBoundWhileInRange() {
        assertTrue(
            RangingDecision.inRange(
                distanceMeters = 3.0f,
                rssiDbm = null,
                currentlyInRange = true,
            )
        )
        assertFalse(
            RangingDecision.inRange(
                distanceMeters = 4.0f,
                rssiDbm = null,
                currentlyInRange = true,
            )
        )
    }

    @Test
    fun distanceUsesStricterEnterBoundWhileOutOfRange() {
        assertFalse(
            RangingDecision.inRange(
                distanceMeters = 3.0f,
                rssiDbm = null,
                currentlyInRange = false,
            )
        )
        assertTrue(
            RangingDecision.inRange(
                distanceMeters = 1.0f,
                rssiDbm = null,
                currentlyInRange = false,
            )
        )
    }

    @Test
    fun rssiFallbackUsesHysteresisBounds() {
        assertTrue(
            RangingDecision.inRange(
                distanceMeters = null,
                rssiDbm = -65,
                currentlyInRange = true,
            )
        )
        assertFalse(
            RangingDecision.inRange(
                distanceMeters = null,
                rssiDbm = -80,
                currentlyInRange = true,
            )
        )
        assertTrue(
            RangingDecision.inRange(
                distanceMeters = null,
                rssiDbm = -60,
                currentlyInRange = false,
            )
        )
    }

    @Test
    fun missingMeasurementFailsClosed() {
        assertFalse(RangingDecision.inRange(null, null, false))
        assertTrue(
            RangingDecision.outOfRange(
                RangingDecision.snapshot(
                    (1..3).map {
                        FfiRangingSample(
                            deviceId = "phone-1",
                            technology = FfiRangingTechnology.UWB,
                            distanceMeters = 9.0,
                            rssiDbm = null,
                            sequence = it.toLong(),
                        )
                    }
                )
            )
        )
        assertFalse(
            RangingDecision.outOfRange(
                RangingDecision.snapshot(emptyList<FfiRangingSample>()).copy(
                    status = FfiProximityStatus.IN_RANGE,
                )
            )
        )
    }

    @Test
    fun macAddressParsesIntoSixBytes() {
        val parsed = ProximitySupervisor.parseMac("AA:BB:CC:DD:EE:FF")
        assertTrue(parsed.size == 6)
        assertTrue(parsed[0] == 0xAA.toByte())
        assertTrue(parsed[5] == 0xFF.toByte())
    }
}
