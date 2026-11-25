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
        if seq == self.last_sequence && observation.device_id.as_str() >= self.last_device_id.as_str() {
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
mod tests {
    use super::*;

    fn observation(device: &str, seq: u64, kind: ContinuityObservationKind) -> AudioContinuityObservation {
        AudioContinuityObservation {
            device_id: device.to_string(),
            peripheral_id: "AA:BB:CC:DD:EE:FF".to_string(),
            kind,
            sequence: seq,
            volume: 72,
            profile: AudioSinkProfile::BapSink,
        }
    }

    #[test]
    fn first_start_claims_and_resumes_without_release() {
        let mut engine = PhoenixContinuityEngine::new();
        let plan = engine
            .submit_observation(observation(
                "phone-1",
                1,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("first start");
        assert_eq!(plan.commands.len(), 2);
        assert_eq!(plan.commands[0].kind, ContinuityCommandKind::Claim);
        assert_eq!(plan.commands[1].kind, ContinuityCommandKind::Resume);
        assert_eq!(engine.owner(), Some("phone-1"));
    }

    #[test]
    fn competing_start_releases_previous_owner_then_claims() {
        let mut engine = PhoenixContinuityEngine::new();
        engine
            .submit_observation(observation(
                "phone-1",
                1,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("first start");
        let plan = engine
            .submit_observation(observation(
                "desktop-1",
                2,
                ContinuityObservationKind::VoiceStarted,
            ))
            .expect("second start");
        assert_eq!(plan.commands.len(), 3);
        assert_eq!(plan.commands[0].kind, ContinuityCommandKind::Release);
        assert_eq!(plan.commands[0].owner_device_id, "phone-1");
        assert_eq!(plan.commands[1].kind, ContinuityCommandKind::Claim);
        assert_eq!(plan.commands[1].owner_device_id, "desktop-1");
        assert_eq!(plan.commands[2].kind, ContinuityCommandKind::Resume);
    }

    #[test]
    fn simultaneous_equal_sequence_uses_stable_device_priority() {
        let mut engine = PhoenixContinuityEngine::new();
        engine
            .submit_observation(observation(
                "z-phone",
                7,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("higher priority device first");
        let plan = engine
            .submit_observation(observation(
                "a-phone",
                7,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("lower device id wins tie");
        assert_eq!(engine.owner(), Some("a-phone"));
        assert_eq!(plan.commands[0].owner_device_id, "z-phone");
        let ignored = engine
            .submit_observation(observation(
                "m-phone",
                7,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("later id loses tie");
        assert!(ignored.commands.is_empty());
    }

    #[test]
    fn stale_observation_cannot_reverse_a_settled_handoff() {
        let mut engine = PhoenixContinuityEngine::new();
        engine
            .submit_observation(observation(
                "phone-1",
                5,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("start");
        let stale = engine
            .submit_observation(observation(
                "desktop-1",
                4,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("stale ignored");
        assert!(stale.commands.is_empty());
        assert_eq!(engine.owner(), Some("phone-1"));
    }

    #[test]
    fn failed_claim_recovers_previous_owner() {
        let mut engine = PhoenixContinuityEngine::new();
        engine
            .submit_observation(observation(
                "phone-1",
                1,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("first start");
        engine
            .submit_observation(observation(
                "desktop-1",
                2,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("handoff");
        let recovery = engine
            .plan_recovery(3, "claim_failed")
            .expect("recovery");
        assert_eq!(recovery.commands.len(), 1);
        assert_eq!(recovery.commands[0].kind, ContinuityCommandKind::AbortAndReclaim);
        assert_eq!(recovery.commands[0].owner_device_id, "phone-1");
        assert_eq!(engine.owner(), Some("phone-1"));
    }

    #[test]
    fn owner_stop_reclaims_previous_owner() {
        let mut engine = PhoenixContinuityEngine::new();
        engine
            .submit_observation(observation(
                "phone-1",
                1,
                ContinuityObservationKind::PlaybackStarted,
            ))
            .expect("first start");
        engine
            .submit_observation(observation(
                "desktop-1",
                2,
                ContinuityObservationKind::VoiceStarted,
            ))
            .expect("handoff");
        let plan = engine
            .submit_observation(observation(
                "desktop-1",
                3,
                ContinuityObservationKind::VoiceStopped,
            ))
            .expect("stop");
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].kind, ContinuityCommandKind::AbortAndReclaim);
        assert_eq!(engine.owner(), None);
    }

    #[test]
    fn invalid_observation_is_rejected_before_state_change() {
        let mut engine = PhoenixContinuityEngine::new();
        let mut bad = observation("", 1, ContinuityObservationKind::PlaybackStarted);
        assert!(matches!(
            engine.submit_observation(bad.clone()),
            Err(KeynearError::Validation { .. })
        ));
        bad.device_id = "phone-1".to_string();
        bad.volume = 101;
        assert!(matches!(
            engine.submit_observation(bad),
            Err(KeynearError::Validation { .. })
        ));
        assert_eq!(engine.owner(), None);
    }
}
