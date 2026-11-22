use keynear_protocol::KeynearError;

use crate::audio::AudioSinkProfile;

pub const MAX_CALL_ID_LEN: usize = 64;
pub const MAX_CALLER_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPhase {
    Incoming,
    Active,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallActionKind {
    Answer,
    Decline,
    Mute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallAlertState {
    pub call_id: String,
    pub caller: String,
    pub phase: CallPhase,
    pub sink_profile: AudioSinkProfile,
    pub sequence: u64,
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CallPlan {
    pub release_sink: bool,
    pub reclaim_sink: bool,
    pub mute: bool,
}

#[derive(Debug, Default)]
pub struct CallAlertEngine {
    state: Option<CallAlertState>,
    last_sequence: u64,
}

impl CallAlertEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Option<&CallAlertState> {
        self.state.as_ref()
    }

    pub fn begin(
        &mut self,
        sequence: u64,
        call_id: &str,
        caller: &str,
        sink_profile: AudioSinkProfile,
    ) -> Result<CallPlan, KeynearError> {
        validate_call_id(call_id)?;
        validate_caller(caller)?;
        if !self.accept(sequence) {
            return Ok(CallPlan::default());
        }
        self.state = Some(CallAlertState {
            call_id: call_id.to_string(),
            caller: caller.to_string(),
            phase: CallPhase::Incoming,
            sink_profile,
            sequence,
            muted: false,
        });
        Ok(CallPlan {
            release_sink: true,
            ..CallPlan::default()
        })
    }

    pub fn apply(
        &mut self,
        sequence: u64,
        call_id: &str,
        action: CallActionKind,
    ) -> Result<CallPlan, KeynearError> {
        if !self.accept(sequence) {
            return Ok(CallPlan::default());
        }
        let expected = match action {
            CallActionKind::Answer | CallActionKind::Decline => CallPhase::Incoming,
            CallActionKind::Mute => CallPhase::Active,
        };
        {
            let state = self.state.as_ref().ok_or_else(|| {
                KeynearError::Protocol("call action requires an active alert".to_string())
            })?;
            if state.call_id != call_id || state.phase != expected {
                return Err(KeynearError::Protocol(
                    "call action targets a stale or different call".to_string(),
                ));
            }
        }
        let state = self.state.as_mut().expect("guard holds a state");
        match action {
            CallActionKind::Answer => {
                state.phase = CallPhase::Active;
                Ok(CallPlan::default())
            }
            CallActionKind::Decline => {
                state.phase = CallPhase::Ended;
                Ok(CallPlan {
                    reclaim_sink: true,
                    ..CallPlan::default()
                })
            }
            CallActionKind::Mute => {
                state.muted = !state.muted;
                Ok(CallPlan {
                    mute: true,
                    ..CallPlan::default()
                })
            }
        }
    }

    pub fn end(&mut self, sequence: u64, call_id: &str) -> Result<CallPlan, KeynearError> {
        if !self.accept(sequence) {
            return Ok(CallPlan::default());
        }
        self.expect_call(call_id)?;
        let state = self.state.as_mut().expect("call guard holds a state");
        let reclaim_sink = state.phase != CallPhase::Ended;
        state.phase = CallPhase::Ended;
        state.muted = false;
        Ok(CallPlan {
            reclaim_sink,
            ..CallPlan::default()
        })
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

    fn expect_call(&self, call_id: &str) -> Result<(), KeynearError> {
        match self.state.as_ref() {
            Some(state) if state.call_id == call_id => Ok(()),
            _ => Err(KeynearError::Protocol(
                "call alert targets a different call".to_string(),
            )),
        }
    }
}

fn validate_call_id(value: &str) -> Result<(), KeynearError> {
    validate("call_id", value, MAX_CALL_ID_LEN)
}

fn validate_caller(value: &str) -> Result<(), KeynearError> {
    validate("caller", value, MAX_CALLER_LEN)
}

fn validate(field: &'static str, value: &str, max: usize) -> Result<(), KeynearError> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(KeynearError::Validation {
            field,
            message: format!("must be 1..{max} safe characters"),
        });
    }
    Ok(())
}
