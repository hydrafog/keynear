use keynear_core::{
    negotiate_capabilities, percent_to_vcp_volume as core_percent_to_vcp,
    vcp_volume_to_percent as core_vcp_to_percent, CAP_AUDIO_CONTINUITY,
    CAP_PRESENCE_GATING, CAP_PROXIMITY_LOCK, PhoenixCapabilities, ProximityConfig,
    ProximityEvaluator, ProximitySnapshot, ProximityStatus, RangingSample, RangingTechnology,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRangingTechnology {
    Uwb,
    BleChannelSounding,
    BleRssi,
}

impl From<FfiRangingTechnology> for RangingTechnology {
    fn from(value: FfiRangingTechnology) -> Self {
        match value {
            FfiRangingTechnology::Uwb => Self::Uwb,
            FfiRangingTechnology::BleChannelSounding => Self::BleChannelSounding,
            FfiRangingTechnology::BleRssi => Self::BleRssi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiProximityStatus {
    Unknown,
    InRange,
    OutOfRange,
}

impl From<ProximityStatus> for FfiProximityStatus {
    fn from(value: ProximityStatus) -> Self {
        match value {
            ProximityStatus::Unknown => Self::Unknown,
            ProximityStatus::InRange => Self::InRange,
            ProximityStatus::OutOfRange => Self::OutOfRange,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiRangingSample {
    pub device_id: String,
    pub technology: FfiRangingTechnology,
    pub distance_meters: Option<f64>,
    pub rssi_dbm: Option<i32>,
    pub sequence: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiProximitySnapshot {
    pub status: FfiProximityStatus,
    pub distance_meters: Option<f64>,
    pub session_locked: bool,
}

impl From<ProximitySnapshot> for FfiProximitySnapshot {
    fn from(value: ProximitySnapshot) -> Self {
        Self {
            status: value.status.into(),
            distance_meters: value.distance_meters.map(f64::from),
            session_locked: value.session_locked,
        }
    }
}

#[uniffi::export]
pub fn phoenix_capability_flags() -> u32 {
    PhoenixCapabilities {
        audio_continuity: true,
        proximity_lock: true,
        presence_gating: true,
    }
    .flags()
}

#[uniffi::export]
pub fn negotiate_phoenix_capabilities(local: u32, peer: u32) -> u32 {
    negotiate_capabilities(local, peer)
}

#[uniffi::export]
pub fn phoenix_capability_bits() -> Vec<u32> {
    vec![CAP_AUDIO_CONTINUITY, CAP_PROXIMITY_LOCK, CAP_PRESENCE_GATING]
}

#[uniffi::export]
pub fn vcp_volume_to_percent(volume: u16) -> u8 {
    core_vcp_to_percent(volume)
}

#[uniffi::export]
pub fn percent_to_vcp_volume(percent: u8) -> u16 {
    core_percent_to_vcp(percent)
}

#[uniffi::export]
pub fn evaluate_ranging_sequence(samples: Vec<FfiRangingSample>) -> FfiProximitySnapshot {
    let Ok(mut evaluator) = ProximityEvaluator::new(ProximityConfig::default()) else {
        return FfiProximitySnapshot {
            status: FfiProximityStatus::Unknown,
            distance_meters: None,
            session_locked: false,
        };
    };
    for sample in samples {
        let _ = evaluator.update(RangingSample {
            device_id: sample.device_id,
            technology: sample.technology.into(),
            distance_meters: sample.distance_meters.map(|value| value as f32),
            rssi_dbm: sample.rssi_dbm,
            sequence: sample.sequence,
        });
    }
    evaluator.snapshot().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_flags_and_bits_are_exported() {
        assert_eq!(
            phoenix_capability_flags(),
            CAP_AUDIO_CONTINUITY | CAP_PROXIMITY_LOCK | CAP_PRESENCE_GATING
        );
        assert_eq!(phoenix_capability_bits().len(), 3);
        assert_eq!(
            negotiate_phoenix_capabilities(phoenix_capability_flags(), CAP_PROXIMITY_LOCK),
            CAP_PROXIMITY_LOCK
        );
    }

    #[test]
    fn volume_helpers_roundtrip() {
        assert_eq!(vcp_volume_to_percent(255), 100);
        assert_eq!(percent_to_vcp_volume(100), 255);
    }

    #[test]
    fn ranging_sequence_crosses_the_exit_bound() {
        let samples = (1..=3)
            .map(|sequence| FfiRangingSample {
                device_id: "phone-1".to_string(),
                technology: FfiRangingTechnology::Uwb,
                distance_meters: Some(9.0),
                rssi_dbm: None,
                sequence,
            })
            .collect();
        let snapshot = evaluate_ranging_sequence(samples);
        assert_eq!(snapshot.status, FfiProximityStatus::OutOfRange);
    }
}
