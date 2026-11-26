package org.keynear.app.proximity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ble.KeynearBleConstants
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
        assertFalse(RangingDecision.inRange(distanceMeters = -1f, rssiDbm = null, currentlyInRange = false))
    }

    @Test
    fun macAddressParsesIntoSixBytes() {
        val parsed = ProximitySupervisor.parseMac("AA:BB:CC:DD:EE:FF")
        assertTrue(parsed.size == 6)
        assertTrue(parsed[0] == 0xAA.toByte())
        assertTrue(parsed[5] == 0xFF.toByte())
    }

    @Test
    fun advertisedFlagsIncludeProximityRangingAndPresenceGating() {
        val flags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS
        assertTrue((flags and KeynearBleConstants.CAP_PROXIMITY_LOCK) != 0u)
        assertTrue((flags and KeynearBleConstants.CAP_RANGING_UWB) != 0u)
        assertTrue((flags and KeynearBleConstants.CAP_PRESENCE_GATING) != 0u)
    }

    @Test
    fun unavailableUwbFallsBackToBleRssi() {
        assertEquals(FfiRangingTechnology.BLE_RSSI, RangingDecision.selectTechnology(false))
        assertEquals(FfiRangingTechnology.UWB, RangingDecision.selectTechnology(true))
        var fallbacks = 0
        val callback = RangingCallback("phone-1", { }, { fallbacks += 1 })
        callback.onOpened()
        assertEquals(0, fallbacks)
        callback.onOpenFailed(1)
        callback.onClosed(1)
        assertEquals(2, fallbacks)
    }
}
