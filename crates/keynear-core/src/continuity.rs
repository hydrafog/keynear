use keynear_protocol::KeynearError;

use crate::audio::AudioSinkProfile;

pub const MAX_DEVICE_ID_LEN: usize = 64;
pub const MAX_REASON_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuityObservationKind {
    PlaybackStarted,
    VoiceStarted,
    PlaybackStopped,
    VoiceStopped,
}

impl ContinuityObservationKind {
    pub fn is_start(self) -> bool {
        matches!(self, Self::PlaybackStarted | Self::VoiceStarted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioContinuityObservation {
    pub device_id: String,
    pub peripheral_id: String,
    pub kind: ContinuityObservationKind,
    pub sequence: u64,
    pub volume: u8,
    pub profile: AudioSinkProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuityCommandKind {
    Release,
    Claim,
    Resume,
    AbortAndReclaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioContinuityCommand {
    pub owner_device_id: String,
    pub peripheral_id: String,
    pub kind: ContinuityCommandKind,
    pub sequence: u64,
    pub volume: u8,
    pub profile: AudioSinkProfile,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContinuityPlan {
    pub commands: Vec<AudioContinuityCommand>,
}

#[derive(Debug, Default)]
pub struct PhoenixContinuityEngine {
    last_sequence: u64,
    last_device_id: String,
    owner: Option<String>,
    previous_owner: Option<String>,
}

impl PhoenixContinuityEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    pub fn submit_observation(
        &mut self,
        observation: AudioContinuityObservation,
    ) -> Result<ContinuityPlan, KeynearError> {
        validate_id("device_id", &observation.device_id)?;
        validate_id("peripheral_id", &observation.peripheral_id)?;
        if observation.volume > 100 {
            return Err(KeynearError::Validation {
                field: "volume",
                message: "continuity volume must be a percentage from 0 to 100".to_string(),
            });
        }
        if !self.accept(&observation) {
            return Ok(ContinuityPlan::default());
        }

        if !observation.kind.is_start() {
            return Ok(self.handle_stop(&observation));
        }

        if self.owner.as_deref() == Some(observation.device_id.as_str()) {
            return Ok(ContinuityPlan::default());
        }

        self.previous_owner = self.owner.take();
        self.owner = Some(observation.device_id.clone());
        let base = &observation;
        let mut commands = Vec::with_capacity(3);
        if let Some(previous) = &self.previous_owner {
            commands.push(command(
                previous,
                base,
                ContinuityCommandKind::Release,
                "handoff_release",
            ));
        }
        commands.push(command(
            &base.device_id,
            base,
            ContinuityCommandKind::Claim,
            "handoff_claim",
        ));
        commands.push(command(
            &base.device_id,
            base,
            ContinuityCommandKind::Resume,
            "handoff_resume",
        ));
        Ok(ContinuityPlan { commands })
    }

    pub fn plan_recovery(
        &mut self,
        sequence: u64,
        reason: &str,
    ) -> Result<ContinuityPlan, KeynearError> {
        let previous = self
            .previous_owner
            .clone()
            .or_else(|| self.owner.clone())
            .ok_or_else(|| KeynearError::Protocol("no owner to reclaim".to_string()))?;
        if sequence <= self.last_sequence {
            return Ok(ContinuityPlan::default());
        }
        self.last_sequence = sequence;
        self.owner = Some(previous.clone());
        let command = AudioContinuityCommand {
            owner_device_id: previous,
            peripheral_id: String::new(),
            kind: ContinuityCommandKind::AbortAndReclaim,
            sequence,
            volume: 0,
            profile: AudioSinkProfile::BapSink,
            reason: truncate_reason(reason),
        };
        Ok(ContinuityPlan {
            commands: vec![command],
        })
    }

    fn handle_stop(&mut self, observation: &AudioContinuityObservation) -> ContinuityPlan {
        if self.owner.as_deref() != Some(observation.device_id.as_str()) {
            return ContinuityPlan::default();
        }
        self.owner = None;
        let Some(previous) = self.previous_owner.take() else {
            return ContinuityPlan::default();
        };
        ContinuityPlan {
            commands: vec![command(
                &previous,
                observation,
                ContinuityCommandKind::AbortAndReclaim,
                "owner_stopped",
            )],
        }
    }

    fn accept(&mut self, observation: &AudioContinuityObservation) -> bool {
        let seq = observation.sequence;
        if seq < self.last_sequence {
            return false;
        }
        if seq == self.last_sequence
            && observation.device_id.as_str() >= self.last_device_id.as_str()
        {
            return false;
        }
        self.last_sequence = seq;
        self.last_device_id = observation.device_id.clone();
        true
    }
}

fn command(
    owner: &str,
    base: &AudioContinuityObservation,
    kind: ContinuityCommandKind,
    reason: &str,
) -> AudioContinuityCommand {
    AudioContinuityCommand {
        owner_device_id: owner.to_string(),
        peripheral_id: base.peripheral_id.clone(),
        kind,
        sequence: base.sequence,
        volume: base.volume,
        profile: base.profile,
        reason: reason.to_string(),
    }
}

fn truncate_reason(reason: &str) -> String {
    let mut value = reason.to_string();
    value.truncate(MAX_REASON_LEN);
    value
}

fn validate_id(field: &'static str, value: &str) -> Result<(), KeynearError> {
    let invalid = value.is_empty()
        || value.len() > MAX_DEVICE_ID_LEN
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'));
    if invalid {
        return Err(KeynearError::Validation {
            field,
            message: format!("{field} must be 1..{MAX_DEVICE_ID_LEN} safe characters"),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "continuity/tests.rs"]
mod tests;
