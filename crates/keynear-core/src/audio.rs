use keynear_protocol::KeynearError;

pub const MAX_PERIPHERAL_ID_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSinkProfile {
    BapSink,
    A2dpSink,
    HfpHf,
}

impl AudioSinkProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bap_sink" => Some(Self::BapSink),
            "a2dp_sink" => Some(Self::A2dpSink),
            "hfp_hf" => Some(Self::HfpHf),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BapSink => "bap_sink",
            Self::A2dpSink => "a2dp_sink",
            Self::HfpHf => "hfp_hf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandoffPhase {
    Idle,
    Prepare,
    Released,
    Claimed,
    Resumed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioHandoffState {
    pub peripheral_id: String,
    pub profile: AudioSinkProfile,
    pub phase: HandoffPhase,
    pub playback_paused: bool,
    pub sink_released: bool,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioHandoffPlan {
    pub pause_playback: bool,
    pub release_sink: bool,
    pub reclaim_sink: bool,
    pub resume_playback: bool,
}

#[derive(Debug, Default)]
pub struct AudioHandoffEngine {
    state: Option<AudioHandoffState>,
    last_sequence: u64,
}

impl AudioHandoffEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Option<&AudioHandoffState> {
        self.state.as_ref()
    }

    pub fn begin(
        &mut self,
        sequence: u64,
        peripheral_id: &str,
        profile: AudioSinkProfile,
    ) -> Result<AudioHandoffPlan, KeynearError> {
        validate_peripheral_id(peripheral_id)?;
        if !self.accept(sequence) {
            return Ok(AudioHandoffPlan::default());
        }
        self.state = Some(AudioHandoffState {
            peripheral_id: peripheral_id.to_string(),
            profile,
            phase: HandoffPhase::Prepare,
            playback_paused: true,
            sink_released: false,
            sequence,
        });
        Ok(AudioHandoffPlan {
            pause_playback: true,
            release_sink: true,
            ..AudioHandoffPlan::default()
        })
    }

    pub fn mark_released(&mut self, sequence: u64) -> Result<(), KeynearError> {
        if !self.accept(sequence) {
            return Ok(());
        }
        self.expect_phase(HandoffPhase::Prepare)?;
        let state = self.state.as_mut().expect("phase guard holds a state");
        state.phase = HandoffPhase::Released;
        state.sink_released = true;
        Ok(())
    }

    pub fn peer_claimed(&mut self, sequence: u64) -> Result<(), KeynearError> {
        if !self.accept(sequence) {
            return Ok(());
        }
        self.expect_phase(HandoffPhase::Released)?;
        let state = self.state.as_mut().expect("phase guard holds a state");
        state.phase = HandoffPhase::Claimed;
        Ok(())
    }

    pub fn release(&mut self, sequence: u64) -> Result<AudioHandoffPlan, KeynearError> {
        if !self.accept(sequence) {
            return Ok(AudioHandoffPlan::default());
        }
        self.expect_phase(HandoffPhase::Claimed)?;
        let state = self.state.as_mut().expect("phase guard holds a state");
        state.phase = HandoffPhase::Resumed;
        state.playback_paused = false;
        state.sink_released = false;
        Ok(AudioHandoffPlan {
            reclaim_sink: true,
            resume_playback: true,
            ..AudioHandoffPlan::default()
        })
    }

    pub fn abort(&mut self, sequence: u64) -> Option<AudioHandoffPlan> {
        if !self.accept(sequence) {
            return None;
        }
        let state = self.state.as_mut()?;
        let plan = AudioHandoffPlan {
            reclaim_sink: state.sink_released,
            resume_playback: state.playback_paused,
            ..AudioHandoffPlan::default()
        };
        state.phase = HandoffPhase::Aborted;
        state.playback_paused = false;
        state.sink_released = false;
        Some(plan)
    }

    pub fn clear(&mut self) {
        self.state = None;
    }

    fn accept(&mut self, sequence: u64) -> bool {
        if sequence <= self.last_sequence {
            return false;
        }
        self.last_sequence = sequence;
        true
    }

    fn expect_phase(&self, expected: HandoffPhase) -> Result<(), KeynearError> {
        match self.state.as_ref() {
            Some(state) if state.phase == expected => Ok(()),
            _ => Err(KeynearError::Protocol(format!(
                "audio handoff expected phase {expected:?}"
            ))),
        }
    }
}

fn validate_peripheral_id(value: &str) -> Result<(), KeynearError> {
    let invalid = value.is_empty()
        || value.len() > MAX_PERIPHERAL_ID_LEN
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'));
    if invalid {
        return Err(KeynearError::Validation {
            field: "peripheral_id",
            message: format!("peripheral id must be 1..{MAX_PERIPHERAL_ID_LEN} safe characters"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn begin(engine: &mut AudioHandoffEngine, seq: u64) -> AudioHandoffPlan {
        engine
            .begin(seq, "AA:BB:CC:DD:EE:FF", AudioSinkProfile::A2dpSink)
            .expect("begin")
    }

    #[test]
    fn handoff_pauses_before_release_then_resumes() {
        let mut engine = AudioHandoffEngine::new();
        let start = begin(&mut engine, 1);
        assert!(start.pause_playback && start.release_sink);
        engine.mark_released(2).expect("released");
        assert_eq!(
            engine.snapshot().expect("state").phase,
            HandoffPhase::Released
        );
        engine.peer_claimed(3).expect("claimed");
        let restore = engine.release(4).expect("release");
        assert!(restore.reclaim_sink && restore.resume_playback);
        let state = engine.snapshot().expect("state");
        assert!(!state.playback_paused && !state.sink_released);
    }

    #[test]
    fn abort_restores_paused_playback_and_released_sink() {
        let mut engine = AudioHandoffEngine::new();
        begin(&mut engine, 1);
        engine.mark_released(2).expect("released");
        let plan = engine.abort(3).expect("abort");
        assert!(plan.reclaim_sink && plan.resume_playback);
        assert_eq!(
            engine.snapshot().expect("state").phase,
            HandoffPhase::Aborted
        );
    }

    #[test]
    fn stale_sequence_is_ignored_without_changing_state() {
        let mut engine = AudioHandoffEngine::new();
        begin(&mut engine, 5);
        let stale = begin(&mut engine, 4);
        assert_eq!(stale, AudioHandoffPlan::default());
        assert_eq!(engine.snapshot().expect("state").sequence, 5);
    }

    #[test]
    fn invalid_peripheral_id_is_rejected_before_state_change() {
        let mut engine = AudioHandoffEngine::new();
        let err = engine
            .begin(1, "", AudioSinkProfile::BapSink)
            .expect_err("empty id rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
        assert!(engine.snapshot().is_none());
        let err = engine
            .begin(2, &"x".repeat(65), AudioSinkProfile::BapSink)
            .expect_err("long id rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
        let err = engine
            .begin(3, "bad\nname", AudioSinkProfile::BapSink)
            .expect_err("control char rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
    }

    #[test]
    fn sink_profiles_parse_from_bluez_names() {
        assert_eq!(
            AudioSinkProfile::parse("bap_sink"),
            Some(AudioSinkProfile::BapSink)
        );
        assert_eq!(
            AudioSinkProfile::parse("a2dp_sink"),
            Some(AudioSinkProfile::A2dpSink)
        );
        assert_eq!(
            AudioSinkProfile::parse("hfp_hf"),
            Some(AudioSinkProfile::HfpHf)
        );
        assert_eq!(AudioSinkProfile::parse("unknown"), None);
        assert_eq!(AudioSinkProfile::HfpHf.as_str(), "hfp_hf");
    }

    #[test]
    fn out_of_order_transition_returns_typed_error() {
        let mut engine = AudioHandoffEngine::new();
        let err = engine.mark_released(1).expect_err("no active handoff");
        assert!(matches!(err, KeynearError::Protocol(_)));
    }
}
