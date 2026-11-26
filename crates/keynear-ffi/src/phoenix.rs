use crate::node::OrionNodeHandle;
use crate::FfiError;
use keynear_core::{
    negotiate_capabilities, percent_to_vcp_volume as core_percent_to_vcp,
    vcp_volume_to_percent as core_vcp_to_percent, PhoenixCapabilities, ProximityConfig,
    ProximityEvaluator, ProximitySnapshot, ProximityStatus, RangingSample, RangingTechnology,
    CAP_AUDIO_CONTINUITY, CAP_PRESENCE_GATING, CAP_PROXIMITY_LOCK,
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
    vec![
        CAP_AUDIO_CONTINUITY,
        CAP_PROXIMITY_LOCK,
        CAP_PRESENCE_GATING,
    ]
}

#[uniffi::export]
pub fn vcp_volume_to_percent(volume: u16) -> u8 {
    core_vcp_to_percent(volume)
}

#[uniffi::export]
pub fn percent_to_vcp_volume(percent: u8) -> u16 {
    core_percent_to_vcp(percent)
}

const DEFAULT_DESK_DISTANCE_METERS: f64 = 3.0;

#[uniffi::export]
pub fn evaluate_ranging_sequence_with_distance(
    samples: Vec<FfiRangingSample>,
    distance_meters: f64,
) -> FfiProximitySnapshot {
    let Ok(mut evaluator) =
        ProximityEvaluator::new(ProximityConfig::from_desk_distance(distance_meters as f32))
    else {
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

#[uniffi::export]
pub fn evaluate_ranging_sequence(samples: Vec<FfiRangingSample>) -> FfiProximitySnapshot {
    evaluate_ranging_sequence_with_distance(samples, DEFAULT_DESK_DISTANCE_METERS)
}

#[uniffi::export]
impl OrionNodeHandle {
    pub fn send_ranging_report(
        &self,
        peer_device_id: String,
        sample: FfiRangingSample,
    ) -> Result<(), FfiError> {
        let report = ranging_report_from_sample(sample);
        self.runtime
            .block_on(self.node.send_ranging_report(&peer_device_id, report))
            .map_err(FfiError::from)
    }
}

pub(crate) fn ranging_report_from_sample(
    sample: FfiRangingSample,
) -> keynear_protocol::v1::RangingReport {
    use keynear_protocol::v1;

    let technology = match sample.technology {
        FfiRangingTechnology::Uwb => v1::RangingTechnology::Uwb,
        FfiRangingTechnology::BleChannelSounding => v1::RangingTechnology::BleCs,
        FfiRangingTechnology::BleRssi => v1::RangingTechnology::BleRssi,
    } as i32;
    v1::RangingReport {
        device_id: sample.device_id,
        technology,
        distance_meters: sample.distance_meters.unwrap_or(f64::NAN) as f32,
        rssi_dbm: sample.rssi_dbm.unwrap_or(i32::MIN),
        sequence: sample.sequence,
    }
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

    #[test]
    fn ranging_sequence_honors_the_configured_desk_distance() {
        let samples = || -> Vec<FfiRangingSample> {
            (1..=3)
                .map(|sequence| FfiRangingSample {
                    device_id: "phone-1".to_string(),
                    technology: FfiRangingTechnology::Uwb,
                    distance_meters: Some(5.0),
                    rssi_dbm: None,
                    sequence,
                })
                .collect()
        };
        assert_eq!(
            evaluate_ranging_sequence(samples()).status,
            FfiProximityStatus::OutOfRange
        );
        assert_eq!(
            evaluate_ranging_sequence_with_distance(samples(), 10.0).status,
            FfiProximityStatus::InRange
        );
    }
}
