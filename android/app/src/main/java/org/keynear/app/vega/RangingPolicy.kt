package org.keynear.app.vega

data class RangingGateInput(
    val negotiatedFlags: UInt,
    val sdkInt: Int,
    val hasHardware: Boolean,
    val method: RangingMethod,
)

object RangingPolicy {
    const val RANGING_SDK_FLOOR: Int = 36

    fun isEnabled(input: RangingGateInput): Boolean {
        if (input.sdkInt < RANGING_SDK_FLOOR) return false
        if (!input.hasHardware) return false
        return when (input.method) {
            RangingMethod.BLE_CHANNEL_SOUNDING -> input.negotiatedFlags and CAP_RANGING_CS != 0u
            RangingMethod.UWB -> input.negotiatedFlags and CAP_RANGING_UWB != 0u
            RangingMethod.NONE -> false
        }
    }
}
