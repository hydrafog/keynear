package org.keynear.app.vega

const val CAP_FILE_TRANSFER: UInt = 0x01u
const val CAP_CLIPBOARD: UInt = 0x02u
const val CAP_LEGACY_RESERVED: UInt = 0x04u
const val CAP_AUDIO_HANDOFF: UInt = 0x08u
const val CAP_RANGING_CS: UInt = 0x10u
const val CAP_RANGING_UWB: UInt = 0x20u
const val CAP_WIFI_AWARE: UInt = 0x40u
const val CAP_WIFI_DIRECT_R2: UInt = 0x80u

enum class RangingMethod {
    NONE,
    BLE_CHANNEL_SOUNDING,
    UWB,
}

data class VegaCapabilities(
    val audioHandoff: Boolean = false,
    val rangingCs: Boolean = false,
    val rangingUwb: Boolean = false,
    val wifiAware: Boolean = false,
    val wifiDirectR2: Boolean = false,
) {
    fun flags(): UInt {
        var flags = 0u
        if (audioHandoff) flags = flags or CAP_AUDIO_HANDOFF
        if (rangingCs) flags = flags or CAP_RANGING_CS
        if (rangingUwb) flags = flags or CAP_RANGING_UWB
        if (wifiAware) flags = flags or CAP_WIFI_AWARE
        if (wifiDirectR2) flags = flags or CAP_WIFI_DIRECT_R2
        return flags
    }
}

object VegaCapability {
    fun negotiate(local: UInt, peer: UInt): UInt = local and peer

    fun rangingMethod(flags: UInt): RangingMethod = when {
        flags and CAP_RANGING_CS != 0u -> RangingMethod.BLE_CHANNEL_SOUNDING
        flags and CAP_RANGING_UWB != 0u -> RangingMethod.UWB
        else -> RangingMethod.NONE
    }
}
