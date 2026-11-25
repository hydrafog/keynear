use keynear_protocol::KeynearError;

pub const MAX_RANGE_METERS: f32 = 200.0;
pub const MIN_RSSI_DBM: i32 = -127;
pub const MAX_RSSI_DBM: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangingTechnology {
    Uwb,
    BleChannelSounding,
    BleRssi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProximityStatus {
    Unknown,
    InRange,
    OutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProximityConfig {
    pub enter_distance_meters: f32,
    pub exit_distance_meters: f32,
    pub enter_rssi_dbm: i32,
    pub exit_rssi_dbm: i32,
    pub smoothing_alpha: f32,
    pub samples_to_switch: u8,
}

impl Default for ProximityConfig {
    fn default() -> Self {
        Self {
            enter_distance_meters: 2.0,
            exit_distance_meters: 3.5,
            enter_rssi_dbm: -62,
            exit_rssi_dbm: -72,
            smoothing_alpha: 0.35,
            samples_to_switch: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangingSample {
    pub device_id: String,
    pub technology: RangingTechnology,
    pub distance_meters: Option<f32>,
    pub rssi_dbm: Option<i32>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProximitySnapshot {
    pub status: ProximityStatus,
    pub distance_meters: Option<f32>,
    pub session_locked: bool,
}

#[derive(Debug, Clone)]
pub struct ProximityEvaluator {
    config: ProximityConfig,
    status: ProximityStatus,
    distance_meters: Option<f32>,
    smoothed_rssi_dbm: Option<f32>,
    pending_in_range: Option<bool>,
    pending_samples: u8,
    session_locked: bool,
    last_sequence: u64,
}

impl ProximityEvaluator {
    pub fn new(config: ProximityConfig) -> Result<Self, KeynearError> {
        validate_config(&config)?;
        Ok(Self {
            config,
            status: ProximityStatus::Unknown,
            distance_meters: None,
            smoothed_rssi_dbm: None,
            pending_in_range: None,
            pending_samples: 0,
            session_locked: false,
            last_sequence: 0,
        })
    }

    pub fn snapshot(&self) -> ProximitySnapshot {
        ProximitySnapshot {
            status: self.status,
            distance_meters: self.distance_meters,
            session_locked: self.session_locked,
        }
    }

    pub fn set_session_locked(&mut self, locked: bool) {
        self.session_locked = locked;
    }

    pub fn update(&mut self, sample: RangingSample) -> Result<ProximitySnapshot, KeynearError> {
        validate_sample(&sample)?;
        if sample.sequence <= self.last_sequence {
            return Ok(self.snapshot());
        }
        self.last_sequence = sample.sequence;
        let in_range = self.range_decision(&sample)?;
        self.apply_hysteresis(in_range);
        Ok(self.snapshot())
    }

    fn range_decision(&mut self, sample: &RangingSample) -> Result<bool, KeynearError> {
        let uses_distance = matches!(
            sample.technology,
            RangingTechnology::Uwb | RangingTechnology::BleChannelSounding
        );
        if uses_distance {
            let distance = sample
                .distance_meters
                .ok_or_else(|| KeynearError::Validation {
                    field: "distance_meters",
                    message: "distance-based ranging requires a distance".to_string(),
                })?;
            self.distance_meters = Some(distance);
            let threshold = if self.status == ProximityStatus::InRange {
                self.config.exit_distance_meters
            } else {
                self.config.enter_distance_meters
            };
            return Ok(distance <= threshold);
        }
        let rssi = sample.rssi_dbm.ok_or_else(|| KeynearError::Validation {
            field: "rssi_dbm",
            message: "RSSI ranging requires an RSSI reading".to_string(),
        })?;
        let reading = match self.smoothed_rssi_dbm {
            Some(previous) => {
                previous * (1.0 - self.config.smoothing_alpha)
                    + rssi as f32 * self.config.smoothing_alpha
            }
            None => rssi as f32,
        };
        self.smoothed_rssi_dbm = Some(reading);
        let threshold = if self.status == ProximityStatus::InRange {
            self.config.exit_rssi_dbm
        } else {
            self.config.enter_rssi_dbm
        };
        Ok(reading >= threshold as f32)
    }

    fn apply_hysteresis(&mut self, in_range: bool) {
        if self.status == ProximityStatus::Unknown {
            self.status = if in_range {
                ProximityStatus::InRange
            } else {
                ProximityStatus::OutOfRange
            };
            self.pending_in_range = Some(in_range);
            self.pending_samples = 1;
            return;
        }
        if self.pending_in_range == Some(in_range) {
            self.pending_samples += 1;
        } else {
            self.pending_in_range = Some(in_range);
            self.pending_samples = 1;
        }
        if self.pending_samples < self.config.samples_to_switch {
            return;
        }
        self.status = if in_range {
            ProximityStatus::InRange
        } else {
            ProximityStatus::OutOfRange
        };
        self.pending_samples = 0;
    }
}

fn validate_sample(sample: &RangingSample) -> Result<(), KeynearError> {
    if sample.device_id.trim().is_empty() {
        return Err(KeynearError::Validation {
            field: "device_id",
            message: "ranging device id must not be empty".to_string(),
        });
    }
    if let Some(distance) = sample.distance_meters {
        if !distance.is_finite() || distance < 0.0 || distance > MAX_RANGE_METERS {
            return Err(KeynearError::Validation {
                field: "distance_meters",
                message: format!("distance must be 0..{MAX_RANGE_METERS} meters"),
            });
        }
    }
    if let Some(rssi) = sample.rssi_dbm {
        if !(MIN_RSSI_DBM..=MAX_RSSI_DBM).contains(&rssi) {
            return Err(KeynearError::Validation {
                field: "rssi_dbm",
                message: format!("RSSI must be {MIN_RSSI_DBM}..{MAX_RSSI_DBM} dBm"),
            });
        }
    }
    Ok(())
}

fn validate_config(config: &ProximityConfig) -> Result<(), KeynearError> {
    if !config.enter_distance_meters.is_finite()
        || !config.exit_distance_meters.is_finite()
        || config.enter_distance_meters <= 0.0
        || config.exit_distance_meters < config.enter_distance_meters
        || config.exit_distance_meters > MAX_RANGE_METERS
    {
        return Err(KeynearError::Validation {
            field: "exit_distance_meters",
            message: "distance bounds must be positive and exit must not precede enter".to_string(),
        });
    }
    if !(MIN_RSSI_DBM..=MAX_RSSI_DBM).contains(&config.enter_rssi_dbm)
        || !(MIN_RSSI_DBM..=MAX_RSSI_DBM).contains(&config.exit_rssi_dbm)
        || config.exit_rssi_dbm > config.enter_rssi_dbm
    {
        return Err(KeynearError::Validation {
            field: "exit_rssi_dbm",
            message: "RSSI bounds must be ordered from exit to enter".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&config.smoothing_alpha) {
        return Err(KeynearError::Validation {
            field: "smoothing_alpha",
            message: "smoothing alpha must be 0..1".to_string(),
        });
    }
    if config.samples_to_switch == 0 {
        return Err(KeynearError::Validation {
            field: "samples_to_switch",
            message: "hysteresis requires at least one sample".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        seq: u64,
        technology: RangingTechnology,
        distance: Option<f32>,
        rssi: Option<i32>,
    ) -> RangingSample {
        RangingSample {
            device_id: "phone-1".to_string(),
            technology,
            distance_meters: distance,
            rssi_dbm: rssi,
            sequence: seq,
        }
    }

    #[test]
    fn uwb_distance_crosses_the_exit_bound_only_after_hysteresis() {
        let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
        assert_eq!(
            evaluator
                .update(sample(1, RangingTechnology::Uwb, Some(1.0), None))
                .unwrap()
                .status,
            ProximityStatus::InRange
        );
        let near_boundary = evaluator
            .update(sample(2, RangingTechnology::Uwb, Some(3.0), None))
            .unwrap();
        assert_eq!(near_boundary.status, ProximityStatus::InRange);
        let outside = evaluator
            .update(sample(3, RangingTechnology::Uwb, Some(9.0), None))
            .unwrap();
        assert_eq!(outside.status, ProximityStatus::InRange);
        let confirmed = evaluator
            .update(sample(4, RangingTechnology::Uwb, Some(9.0), None))
            .unwrap();
        assert_eq!(confirmed.status, ProximityStatus::OutOfRange);
    }

    #[test]
    fn entering_range_requires_the_stricter_enter_bound() {
        let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
        for seq in 1..=2 {
            evaluator
                .update(sample(seq, RangingTechnology::Uwb, Some(8.0), None))
                .unwrap();
        }
        assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
        evaluator
            .update(sample(3, RangingTechnology::Uwb, Some(3.0), None))
            .unwrap();
        assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
        for seq in 4..=5 {
            evaluator
                .update(sample(seq, RangingTechnology::Uwb, Some(1.0), None))
                .unwrap();
        }
        assert_eq!(evaluator.snapshot().status, ProximityStatus::InRange);
    }

    #[test]
    fn ble_rssi_is_smoothed_before_comparison() {
        let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
        evaluator
            .update(sample(1, RangingTechnology::BleRssi, None, Some(-60)))
            .unwrap();
        let weak = evaluator
            .update(sample(2, RangingTechnology::BleRssi, None, Some(-90)))
            .unwrap();
        assert_eq!(weak.status, ProximityStatus::InRange);
        for seq in 3..=4 {
            evaluator
                .update(sample(seq, RangingTechnology::BleRssi, None, Some(-90)))
                .unwrap();
        }
        assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
    }

    #[test]
    fn stale_and_invalid_samples_are_rejected_without_state_change() {
        let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
        evaluator
            .update(sample(5, RangingTechnology::Uwb, Some(1.0), None))
            .unwrap();
        evaluator
            .update(sample(4, RangingTechnology::Uwb, Some(50.0), None))
            .unwrap();
        assert_eq!(evaluator.snapshot().distance_meters, Some(1.0));
        let err = evaluator
            .update(sample(6, RangingTechnology::Uwb, Some(f32::NAN), None))
            .expect_err("NaN rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
        let err = evaluator
            .update(sample(7, RangingTechnology::Uwb, None, None))
            .expect_err("missing distance rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
    }

    #[test]
    fn session_lock_is_recorded_without_changing_range() {
        let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
        evaluator
            .update(sample(1, RangingTechnology::Uwb, Some(1.0), None))
            .unwrap();
        evaluator.set_session_locked(true);
        assert!(evaluator.snapshot().session_locked);
        assert_eq!(evaluator.snapshot().status, ProximityStatus::InRange);
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        let config = ProximityConfig {
            exit_distance_meters: 1.0,
            enter_distance_meters: 2.0,
            ..ProximityConfig::default()
        };
        assert!(matches!(
            ProximityEvaluator::new(config),
            Err(KeynearError::Validation { .. })
        ));
    }
}
