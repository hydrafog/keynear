package org.keynear.app.vega

enum class FallbackMedia {
    NONE,
    WIFI_AWARE_NAN,
    WIFI_DIRECT_R2,
}

enum class ProbeStatus {
    UNAVAILABLE,
    AVAILABLE,
    DISALLOWED,
}

data class FallbackProbe(val media: FallbackMedia, val status: ProbeStatus)

data class FallbackChoice(val media: FallbackMedia, val status: ProbeStatus)

object FallbackSelection {
    fun select(aware: FallbackProbe, direct: FallbackProbe): FallbackChoice {
        if (aware.status == ProbeStatus.AVAILABLE) {
            return FallbackChoice(FallbackMedia.WIFI_AWARE_NAN, ProbeStatus.AVAILABLE)
        }
        if (direct.status == ProbeStatus.AVAILABLE) {
            return FallbackChoice(FallbackMedia.WIFI_DIRECT_R2, ProbeStatus.AVAILABLE)
        }
        return FallbackChoice(FallbackMedia.NONE, ProbeStatus.UNAVAILABLE)
    }
}
