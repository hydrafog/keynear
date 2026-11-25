use keynear_protocol::KeynearError;

pub const MAX_PERIPHERAL_ID_LEN: usize = 64;
pub const VCP_VOLUME_MIN: u16 = 0;
pub const VCP_VOLUME_MAX: u16 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSinkProfile {
    BapSink,
    BapSource,
    A2dpSink,
    HfpHf,
}

impl AudioSinkProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bap_sink" => Some(Self::BapSink),
            "bap_source" => Some(Self::BapSource),
            "a2dp_sink" => Some(Self::A2dpSink),
            "hfp_hf" => Some(Self::HfpHf),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BapSink => "bap_sink",
            Self::BapSource => "bap_source",
            Self::A2dpSink => "a2dp_sink",
            Self::HfpHf => "hfp_hf",
        }
    }
}

pub fn vcp_volume_to_percent(volume: u16) -> u8 {
    let clamped = volume.clamp(VCP_VOLUME_MIN, VCP_VOLUME_MAX);
    ((u32::from(clamped) * 100 + u32::from(VCP_VOLUME_MAX) / 2) / u32::from(VCP_VOLUME_MAX)) as u8
}

pub fn percent_to_vcp_volume(percent: u8) -> u16 {
    let clamped = percent.min(100);
    ((u32::from(clamped) * u32::from(VCP_VOLUME_MAX) + 50) / 100) as u16
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
#[path = "audio/tests.rs"]
mod tests;
