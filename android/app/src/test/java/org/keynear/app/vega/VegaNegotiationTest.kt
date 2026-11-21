package org.keynear.app.vega

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VegaNegotiationTest {
    @Test
    fun capabilityFlagsLeaveDracoBitsUntouched() {
        assertEquals(0x01u, CAP_FILE_TRANSFER)
        assertEquals(0x02u, CAP_CLIPBOARD)
        assertEquals(0x04u, CAP_LEGACY_RESERVED)
        val flags = VegaCapabilities(audioHandoff = true, wifiAware = true).flags()
        assertEquals(CAP_AUDIO_HANDOFF or CAP_WIFI_AWARE, flags)
        assertEquals(0u, flags and 0x07u)
    }

    @Test
    fun negotiationIntersectsOnlyAdvertisedBits() {
        val local = VegaCapabilities(audioHandoff = true, rangingCs = true).flags()
        val peer = VegaCapabilities(audioHandoff = true, rangingUwb = true).flags()
        val negotiated = VegaCapability.negotiate(local, peer)
        assertEquals(CAP_AUDIO_HANDOFF, negotiated)
        assertEquals(RangingMethod.NONE, VegaCapability.rangingMethod(negotiated))
    }

    @Test
    fun rangingRequiresCapabilityHardwareAndSdk() {
        val flags = CAP_RANGING_CS or CAP_RANGING_UWB
        val enabled = RangingGateInput(flags, 36, true, RangingMethod.BLE_CHANNEL_SOUNDING)
        assertTrue(RangingPolicy.isEnabled(enabled))
        assertFalse(RangingPolicy.isEnabled(enabled.copy(sdkInt = 35)))
        assertFalse(RangingPolicy.isEnabled(enabled.copy(hasHardware = false)))
        assertFalse(
            RangingPolicy.isEnabled(
                RangingGateInput(0u, 36, true, RangingMethod.UWB),
            ),
        )
        assertFalse(RangingPolicy.isEnabled(enabled.copy(method = RangingMethod.NONE)))
    }

    @Test
    fun awareIsPreferredOverDirect() {
        val aware = FallbackProbe(FallbackMedia.WIFI_AWARE_NAN, ProbeStatus.AVAILABLE)
        val direct = FallbackProbe(FallbackMedia.WIFI_DIRECT_R2, ProbeStatus.AVAILABLE)
        assertEquals(FallbackMedia.WIFI_AWARE_NAN, FallbackSelection.select(aware, direct).media)
        assertEquals(
            FallbackMedia.WIFI_DIRECT_R2,
            FallbackSelection.select(aware.copy(status = ProbeStatus.DISALLOWED), direct).media,
        )
    }

    @Test
    fun noUsableMediumReportsUnavailable() {
        val aware = FallbackProbe(FallbackMedia.WIFI_AWARE_NAN, ProbeStatus.UNAVAILABLE)
        val direct = FallbackProbe(FallbackMedia.WIFI_DIRECT_R2, ProbeStatus.DISALLOWED)
        val choice = FallbackSelection.select(aware, direct)
        assertEquals(FallbackMedia.NONE, choice.media)
        assertEquals(ProbeStatus.UNAVAILABLE, choice.status)
    }
}
