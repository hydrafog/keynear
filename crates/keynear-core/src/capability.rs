pub const CAP_FILE_TRANSFER: u32 = 0x01;
pub const CAP_CLIPBOARD: u32 = 0x02;
pub const CAP_LEGACY_RESERVED: u32 = 0x04;
pub const CAP_AUDIO_HANDOFF: u32 = 0x08;
pub const CAP_RANGING_CS: u32 = 0x10;
pub const CAP_RANGING_UWB: u32 = 0x20;
pub const CAP_WIFI_AWARE: u32 = 0x40;
pub const CAP_WIFI_DIRECT_R2: u32 = 0x80;
pub const CAP_RESUMABLE_TRANSFER: u32 = 0x100;
pub const CAP_NOTIFICATION_MIRROR: u32 = 0x200;
pub const CAP_MEDIA_BRIDGE: u32 = 0x400;
pub const CAP_CALL_ALERTS: u32 = 0x800;
pub const CAP_AUDIO_CONTINUITY: u32 = 0x1000;
pub const CAP_PROXIMITY_LOCK: u32 = 0x2000;
pub const CAP_PRESENCE_GATING: u32 = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangingMethod {
    None,
    BleChannelSounding,
    Uwb,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VegaCapabilities {
    pub audio_handoff: bool,
    pub ranging_cs: bool,
    pub ranging_uwb: bool,
    pub wifi_aware: bool,
    pub wifi_direct_r2: bool,
}

impl VegaCapabilities {
    pub fn flags(self) -> u32 {
        let mut flags = 0u32;
        if self.audio_handoff {
            flags |= CAP_AUDIO_HANDOFF;
        }
        if self.ranging_cs {
            flags |= CAP_RANGING_CS;
        }
        if self.ranging_uwb {
            flags |= CAP_RANGING_UWB;
        }
        if self.wifi_aware {
            flags |= CAP_WIFI_AWARE;
        }
        if self.wifi_direct_r2 {
            flags |= CAP_WIFI_DIRECT_R2;
        }
        flags
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhoenixCapabilities {
    pub audio_continuity: bool,
    pub proximity_lock: bool,
    pub presence_gating: bool,
}

impl PhoenixCapabilities {
    pub fn flags(self) -> u32 {
        let mut flags = 0u32;
        if self.audio_continuity {
            flags |= CAP_AUDIO_CONTINUITY;
        }
        if self.proximity_lock {
            flags |= CAP_PROXIMITY_LOCK;
        }
        if self.presence_gating {
            flags |= CAP_PRESENCE_GATING;
        }
        flags
    }
}

pub fn negotiate_capabilities(local: u32, peer: u32) -> u32 {
    local & peer
}

pub fn ranging_method(flags: u32) -> RangingMethod {
    if flags & CAP_RANGING_CS != 0 {
        RangingMethod::BleChannelSounding
    } else if flags & CAP_RANGING_UWB != 0 {
        RangingMethod::Uwb
    } else {
        RangingMethod::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_capability_bits_stay_stable() {
        assert_eq!(CAP_FILE_TRANSFER, 0x01);
        assert_eq!(CAP_CLIPBOARD, 0x02);
        assert_eq!(CAP_LEGACY_RESERVED, 0x04);
        assert_eq!(CAP_AUDIO_HANDOFF, 0x08);
        assert_eq!(CAP_RANGING_CS, 0x10);
        assert_eq!(CAP_RANGING_UWB, 0x20);
        assert_eq!(CAP_WIFI_AWARE, 0x40);
    }

    #[test]
    fn resume_and_continuity_capability_bits_stay_stable() {
        assert_eq!(CAP_WIFI_DIRECT_R2, 0x80);
        assert_eq!(CAP_RESUMABLE_TRANSFER, 0x100);
        assert_eq!(CAP_NOTIFICATION_MIRROR, 0x200);
        assert_eq!(CAP_MEDIA_BRIDGE, 0x400);
        assert_eq!(CAP_CALL_ALERTS, 0x800);
    }

    #[test]
    fn negotiate_intersects_only_advertised_bits() {
        let local = VegaCapabilities {
            audio_handoff: true,
            ranging_cs: true,
            ..VegaCapabilities::default()
        }
        .flags();
        let peer = VegaCapabilities {
            audio_handoff: true,
            ranging_uwb: true,
            ..VegaCapabilities::default()
        }
        .flags();
        let negotiated = negotiate_capabilities(local, peer);
        assert_eq!(negotiated, CAP_AUDIO_HANDOFF);
        assert_eq!(ranging_method(negotiated), RangingMethod::None);
    }

    #[test]
    fn ranging_prefers_channel_sounding_over_uwb() {
        let both = CAP_RANGING_CS | CAP_RANGING_UWB;
        assert_eq!(ranging_method(both), RangingMethod::BleChannelSounding);
        assert_eq!(ranging_method(CAP_RANGING_UWB), RangingMethod::Uwb);
    }

    #[test]
    fn phoenix_capability_bits_stay_stable_and_negotiate() {
        assert_eq!(CAP_AUDIO_CONTINUITY, 0x1000);
        assert_eq!(CAP_PROXIMITY_LOCK, 0x2000);
        assert_eq!(CAP_PRESENCE_GATING, 0x4000);
        let local = PhoenixCapabilities {
            audio_continuity: true,
            proximity_lock: true,
            ..PhoenixCapabilities::default()
        }
        .flags();
        let peer = PhoenixCapabilities {
            audio_continuity: true,
            presence_gating: true,
            ..PhoenixCapabilities::default()
        }
        .flags();
        assert_eq!(negotiate_capabilities(local, peer), CAP_AUDIO_CONTINUITY);
    }

    #[test]
    fn vega_flags_leave_the_low_bits_untouched() {
        let flags = VegaCapabilities {
            audio_handoff: true,
            wifi_aware: true,
            ..VegaCapabilities::default()
        }
        .flags();
        assert_eq!(flags & 0x07, 0);
        assert_eq!(flags, CAP_AUDIO_HANDOFF | CAP_WIFI_AWARE);
    }

    #[test]
    fn flags_build_from_empty_and_full_configurations() {
        assert_eq!(VegaCapabilities::default().flags(), 0);
        let all_vega = VegaCapabilities {
            audio_handoff: true,
            ranging_cs: true,
            ranging_uwb: true,
            wifi_aware: true,
            wifi_direct_r2: true,
        };
        assert_eq!(
            all_vega.flags(),
            CAP_AUDIO_HANDOFF
                | CAP_RANGING_CS
                | CAP_RANGING_UWB
                | CAP_WIFI_AWARE
                | CAP_WIFI_DIRECT_R2
        );
        assert_eq!(PhoenixCapabilities::default().flags(), 0);
        let all_phoenix = PhoenixCapabilities {
            audio_continuity: true,
            proximity_lock: true,
            presence_gating: true,
        };
        assert_eq!(
            all_phoenix.flags(),
            CAP_AUDIO_CONTINUITY | CAP_PROXIMITY_LOCK | CAP_PRESENCE_GATING
        );
    }

    #[test]
    fn negotiate_and_ranging_method_cover_empty_inputs() {
        assert_eq!(negotiate_capabilities(0, CAP_FILE_TRANSFER), 0);
        assert_eq!(negotiate_capabilities(CAP_FILE_TRANSFER, 0), 0);
        assert_eq!(
            negotiate_capabilities(CAP_FILE_TRANSFER, CAP_FILE_TRANSFER),
            CAP_FILE_TRANSFER
        );
        assert_eq!(ranging_method(0), RangingMethod::None);
        assert_eq!(
            ranging_method(CAP_RANGING_CS),
            RangingMethod::BleChannelSounding
        );
        assert_eq!(
            ranging_method(CAP_RANGING_UWB | CAP_FILE_TRANSFER),
            RangingMethod::Uwb
        );
    }
}
