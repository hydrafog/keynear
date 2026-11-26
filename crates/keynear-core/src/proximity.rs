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

impl ProximityConfig {
    pub fn from_desk_distance(exit_distance_meters: f32) -> Self {
        let exit = if exit_distance_meters.is_finite() {
            exit_distance_meters.clamp(0.5, MAX_RANGE_METERS)
        } else {
            Self::default().exit_distance_meters
        };
        let enter = (exit * 0.6).max(0.5);
        let enter_rssi = distance_to_rssi_dbm(enter);
        let exit_rssi = distance_to_rssi_dbm(exit).min(enter_rssi);
        Self {
            enter_distance_meters: enter,
            exit_distance_meters: exit,
            enter_rssi_dbm: enter_rssi,
            exit_rssi_dbm: exit_rssi,
            ..Self::default()
        }
    }
}

fn distance_to_rssi_dbm(distance_meters: f32) -> i32 {
    let clamped = distance_meters.clamp(0.5, MAX_RANGE_METERS);
    let estimated = (-56.0 - 25.0 * clamped.log10()).round() as i32;
    estimated.clamp(MIN_RSSI_DBM, MAX_RSSI_DBM)
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

    pub fn update_config(&mut self, config: ProximityConfig) -> Result<(), KeynearError> {
        validate_config(&config)?;
        self.config = config;
        self.pending_in_range = None;
        self.pending_samples = 0;
        Ok(())
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
#[path = "proximity/tests.rs"]
mod tests;
