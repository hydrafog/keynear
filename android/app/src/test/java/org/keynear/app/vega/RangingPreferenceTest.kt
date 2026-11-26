package org.keynear.app.vega

import org.junit.Assert.assertEquals
import org.junit.Test

class RangingPreferenceTest {
    @Test
    fun preferredMethodPrefersUwbThenChannelSoundingWithRealCapabilityBits() {
        val both = CAP_RANGING_UWB or CAP_RANGING_CS
        assertEquals(RangingMethod.UWB, RangingPolicy.preferredMethod(both, 36, true))
        assertEquals(RangingMethod.BLE_CHANNEL_SOUNDING, RangingPolicy.preferredMethod(both, 36, false))
        assertEquals(RangingMethod.BLE_CHANNEL_SOUNDING, RangingPolicy.preferredMethod(CAP_RANGING_CS, 36, false))
        assertEquals(RangingMethod.NONE, RangingPolicy.preferredMethod(CAP_RANGING_UWB, 36, false))
        assertEquals(RangingMethod.NONE, RangingPolicy.preferredMethod(CAP_RANGING_UWB, 35, true))
        assertEquals(RangingMethod.NONE, RangingPolicy.preferredMethod(0u, 36, true))
    }

    @Test
    fun negotiateAndFlagsExposeOnlyAdvertisedBits() {
        val all = VegaCapabilities(
            audioHandoff = true,
            rangingCs = true,
            rangingUwb = true,
            wifiAware = true,
            wifiDirectR2 = true,
        ).flags()
        assertEquals(
            CAP_AUDIO_HANDOFF or CAP_RANGING_CS or CAP_RANGING_UWB or CAP_WIFI_AWARE or CAP_WIFI_DIRECT_R2,
            all,
        )
        assertEquals(0u, VegaCapabilities().flags())
        assertEquals(0u, all and (CAP_FILE_TRANSFER or CAP_CLIPBOARD or CAP_LEGACY_RESERVED))
        assertEquals(CAP_RANGING_CS, VegaCapability.negotiate(all, CAP_RANGING_CS))
        assertEquals(0u, VegaCapability.negotiate(CAP_RANGING_CS, CAP_RANGING_UWB))
        assertEquals(all, VegaCapability.negotiate(all, all))
    }

    @Test
    fun rangingMethodUsesChannelSoundingPriorityWithRealFlags() {
        assertEquals(RangingMethod.BLE_CHANNEL_SOUNDING, VegaCapability.rangingMethod(CAP_RANGING_CS))
        assertEquals(RangingMethod.UWB, VegaCapability.rangingMethod(CAP_RANGING_UWB))
        assertEquals(
            RangingMethod.BLE_CHANNEL_SOUNDING,
            VegaCapability.rangingMethod(CAP_RANGING_CS or CAP_RANGING_UWB),
        )
        assertEquals(RangingMethod.NONE, VegaCapability.rangingMethod(CAP_AUDIO_HANDOFF))
    }
}
