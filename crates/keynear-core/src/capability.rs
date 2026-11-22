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
    fn perseus_and_carina_capability_bits_stay_stable() {
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
}
