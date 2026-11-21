use keynear_core::{negotiate_capabilities, ranging_method, RangingMethod, VegaCapabilities};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRangingMethod {
    Off,
    BleChannelSounding,
    Uwb,
}

impl From<RangingMethod> for FfiRangingMethod {
    fn from(value: RangingMethod) -> Self {
        match value {
            RangingMethod::None => Self::Off,
            RangingMethod::BleChannelSounding => Self::BleChannelSounding,
            RangingMethod::Uwb => Self::Uwb,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiFallbackMedia {
    NoMedia,
    WifiAwareNan,
    WifiDirectR2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiFallbackStatus {
    Unavailable,
    Available,
    Disallowed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Record)]
pub struct FfiVegaCapabilities {
    pub audio_handoff: bool,
    pub ranging_cs: bool,
    pub ranging_uwb: bool,
    pub wifi_aware: bool,
    pub wifi_direct_r2: bool,
}

impl From<FfiVegaCapabilities> for VegaCapabilities {
    fn from(value: FfiVegaCapabilities) -> Self {
        Self {
            audio_handoff: value.audio_handoff,
            ranging_cs: value.ranging_cs,
            ranging_uwb: value.ranging_uwb,
            wifi_aware: value.wifi_aware,
            wifi_direct_r2: value.wifi_direct_r2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiCapabilityNegotiation {
    pub local_flags: u32,
    pub peer_flags: u32,
    pub negotiated_flags: u32,
    pub preferred_ranging: FfiRangingMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiFallbackSelection {
    pub media: FfiFallbackMedia,
    pub status: FfiFallbackStatus,
}

#[uniffi::export]
pub fn vega_capability_flags(capabilities: FfiVegaCapabilities) -> u32 {
    VegaCapabilities::from(capabilities).flags()
}

#[uniffi::export]
pub fn negotiate_vega_capabilities(local: u32, peer: u32) -> FfiCapabilityNegotiation {
    let negotiated = negotiate_capabilities(local, peer);
    FfiCapabilityNegotiation {
        local_flags: local,
        peer_flags: peer,
        negotiated_flags: negotiated,
        preferred_ranging: ranging_method(negotiated).into(),
    }
}

#[uniffi::export]
pub fn select_vega_fallback(aware_available: bool, direct_available: bool) -> FfiFallbackSelection {
    use keynear_core::{FallbackMedia, FallbackProbeResult, FallbackStatus};
    let aware = FallbackProbeResult {
        media: FallbackMedia::WifiAwareNan,
        status: fallback_status(aware_available),
    };
    let direct = FallbackProbeResult {
        media: FallbackMedia::WifiDirectR2,
        status: fallback_status(direct_available),
    };
    let selected = keynear_core::select_fallback(aware, direct);
    FfiFallbackSelection {
        media: match selected.media {
            FallbackMedia::WifiAwareNan => FfiFallbackMedia::WifiAwareNan,
            FallbackMedia::WifiDirectR2 => FfiFallbackMedia::WifiDirectR2,
            FallbackMedia::None => FfiFallbackMedia::NoMedia,
        },
        status: match selected.status {
            FallbackStatus::Available => FfiFallbackStatus::Available,
            FallbackStatus::Unavailable => FfiFallbackStatus::Unavailable,
            FallbackStatus::Disallowed => FfiFallbackStatus::Disallowed,
        },
    }
}

fn fallback_status(available: bool) -> keynear_core::FallbackStatus {
    if available {
        keynear_core::FallbackStatus::Available
    } else {
        keynear_core::FallbackStatus::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keynear_core::{CAP_AUDIO_HANDOFF, CAP_RANGING_CS, CAP_RANGING_UWB};

    #[test]
    fn exported_flags_and_negotiation_match_core_rules() {
        let flags = vega_capability_flags(FfiVegaCapabilities {
            audio_handoff: true,
            ranging_cs: true,
            ..FfiVegaCapabilities::default()
        });
        assert_eq!(flags, CAP_AUDIO_HANDOFF | CAP_RANGING_CS);
        let negotiated = negotiate_vega_capabilities(flags, CAP_AUDIO_HANDOFF | CAP_RANGING_UWB);
        assert_eq!(negotiated.negotiated_flags, CAP_AUDIO_HANDOFF);
        assert_eq!(negotiated.preferred_ranging, FfiRangingMethod::Off);
    }

    #[test]
    fn exported_fallback_prefers_aware() {
        let selected = select_vega_fallback(true, true);
        assert_eq!(selected.media, FfiFallbackMedia::WifiAwareNan);
        let selected = select_vega_fallback(false, true);
        assert_eq!(selected.media, FfiFallbackMedia::WifiDirectR2);
        let selected = select_vega_fallback(false, false);
        assert_eq!(selected.media, FfiFallbackMedia::NoMedia);
        assert_eq!(selected.status, FfiFallbackStatus::Unavailable);
    }
}
