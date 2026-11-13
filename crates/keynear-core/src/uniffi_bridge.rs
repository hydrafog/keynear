use std::sync::Mutex;

use crate::error::KeynearError;
use crate::model::{DaemonConfig, VisibilityMode};
use crate::proto::{control_frame_to_envelope, decode_control_frame, MAX_CONTROL_FRAME_BYTES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiVisibilityMode {
    Hidden,
    PairedOnly,
    EveryoneNearby,
}

impl From<VisibilityMode> for FfiVisibilityMode {
    fn from(value: VisibilityMode) -> Self {
        match value {
            VisibilityMode::Hidden => Self::Hidden,
            VisibilityMode::PairedOnly => Self::PairedOnly,
            VisibilityMode::EveryoneNearby => Self::EveryoneNearby,
        }
    }
}

impl From<FfiVisibilityMode> for VisibilityMode {
    fn from(value: FfiVisibilityMode) -> Self {
        match value {
            FfiVisibilityMode::Hidden => Self::Hidden,
            FfiVisibilityMode::PairedOnly => Self::PairedOnly,
            FfiVisibilityMode::EveryoneNearby => Self::EveryoneNearby,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("configuration error: {detail}")]
    Config { detail: String },
    #[error("validation error on {field}: {detail}")]
    Validation { field: String, detail: String },
    #[error("control frame size {size} bytes exceeds maximum {max} bytes")]
    FrameTooLarge { size: u64, max: u64 },
    #[error("protocol error: {detail}")]
    Protocol { detail: String },
    #[error("cryptographic error: {detail}")]
    Crypto { detail: String },
    #[error("pairing verification timed out after 120 seconds")]
    PairingExpired,
    #[error("pairing rejected")]
    PairingRejected,
    #[error("keyring error: {detail}")]
    Keyring { detail: String },
    #[error("bluetooth error: {detail}")]
    Bluetooth { detail: String },
    #[error("i/o error: {detail}")]
    Io { detail: String },
}

impl From<KeynearError> for FfiError {
    fn from(err: KeynearError) -> Self {
        match err {
            KeynearError::Config(detail) => Self::Config { detail },
            KeynearError::Validation { field, message } => Self::Validation {
                field: field.to_string(),
                detail: message,
            },
            KeynearError::FrameTooLarge { size, max } => Self::FrameTooLarge {
                size: size as u64,
                max: max as u64,
            },
            KeynearError::Protocol(detail) => Self::Protocol { detail },
            KeynearError::Crypto(detail) => Self::Crypto { detail },
            KeynearError::PairingExpired => Self::PairingExpired,
            KeynearError::PairingRejected => Self::PairingRejected,
            KeynearError::Keyring(detail) => Self::Keyring { detail },
            KeynearError::Bluetooth(detail) => Self::Bluetooth { detail },
            KeynearError::Io(io_err) => Self::Io {
                detail: io_err.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CoreStatus {
    pub version: String,
    pub max_control_frame_bytes: u32,
    pub ready: bool,
}

#[derive(Debug, Default, uniffi::Object)]
pub struct CoreHandle {
    visibility_mode: Mutex<VisibilityMode>,
}

#[uniffi::export]
impl CoreHandle {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            visibility_mode: Mutex::new(VisibilityMode::default()),
        }
    }

    pub fn status(&self) -> CoreStatus {
        get_core_status()
    }

    pub fn visibility_mode(&self) -> FfiVisibilityMode {
        let guard = self
            .visibility_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        FfiVisibilityMode::from(*guard)
    }

    pub fn cycle_visibility_mode(&self) -> FfiVisibilityMode {
        let mut guard = self
            .visibility_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next = match *guard {
            VisibilityMode::Hidden => VisibilityMode::PairedOnly,
            VisibilityMode::PairedOnly => VisibilityMode::EveryoneNearby,
            VisibilityMode::EveryoneNearby => VisibilityMode::Hidden,
        };
        *guard = next;
        FfiVisibilityMode::from(next)
    }

    pub fn validate_config_toml(&self, raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
        let parsed: DaemonConfig =
            toml::from_str(&raw_toml).map_err(|err| KeynearError::Config(err.to_string()))?;
        parsed.validate()?;
        Ok(FfiVisibilityMode::from(parsed.visibility_mode))
    }

    pub fn validate_control_frame(&self, bytes: Vec<u8>) -> Result<String, FfiError> {
        let frame = decode_control_frame(&bytes)?;
        control_frame_to_envelope(&frame)?;
        Ok(frame.kind)
    }
}

#[uniffi::export]
pub fn get_core_status() -> CoreStatus {
    CoreStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        max_control_frame_bytes: MAX_CONTROL_FRAME_BYTES as u32,
        ready: true,
    }
}

#[uniffi::export]
pub fn default_config_summary() -> String {
    let config = DaemonConfig::default();
    format!(
        "visibility={:?},incoming_dir={}",
        config.visibility_mode, config.incoming_directory
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use prost::Message;

    use super::*;
    use crate::proto::{encode_control_frame, envelope_to_control_frame, v1, ControlFrame};

    #[test]
    fn core_status_reports_ready() {
        let handle = CoreHandle::new();
        let status = handle.status();
        assert!(status.ready);
        assert_eq!(
            status.max_control_frame_bytes,
            MAX_CONTROL_FRAME_BYTES as u32
        );
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
    }

    #[test]
    fn core_handle_cycles_visibility_thread_safely() {
        let handle = Arc::new(CoreHandle::new());
        let mut workers = Vec::new();
        for _ in 0..6 {
            let shared = Arc::clone(&handle);
            workers.push(thread::spawn(move || {
                shared.cycle_visibility_mode();
            }));
        }
        for worker in workers {
            worker.join().expect("thread completes");
        }
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
    }

    #[test]
    fn validate_config_toml_checks_schema_and_validation() {
        let handle = CoreHandle::new();
        let mode = handle
            .validate_config_toml("visibilityMode = \"everyoneNearby\"\n".to_string())
            .expect("valid toml succeeds");
        assert_eq!(mode, FfiVisibilityMode::EveryoneNearby);

        let err = handle
            .validate_config_toml("incomingDirectory = \"\"\n".to_string())
            .expect_err("empty incoming directory fails");
        assert!(matches!(err, FfiError::Validation { .. }));
    }

    #[test]
    fn validate_control_frame_checks_envelope_and_size() {
        let handle = CoreHandle::new();
        let env = v1::Envelope {
            version: 1,
            sequence: 11,
            timestamp_ms: 1_763_000_000_000,
            payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
                acked_sequence: 10,
                accepted: true,
                reason: String::new(),
            })),
        };
        let frame = envelope_to_control_frame(&env).expect("envelope to frame");
        let bytes = encode_control_frame(&frame).expect("encode frame");
        let kind = handle
            .validate_control_frame(bytes)
            .expect("valid control frame accepted");
        assert_eq!(kind, "control_ack");

        let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
        assert!(matches!(
            handle.validate_control_frame(oversized),
            Err(FfiError::FrameTooLarge { .. })
        ));

        let empty_payload = v1::ControlAck {
            acked_sequence: 1,
            accepted: true,
            reason: String::new(),
        }
        .encode_to_vec();
        let bad_kind = ControlFrame {
            version: 1,
            sequence: 1,
            timestamp_ms: 1,
            kind: "invalid".to_string(),
            payload: empty_payload,
        };
        let bad_bytes = encode_control_frame(&bad_kind).expect("encode bad kind frame");
        assert!(matches!(
            handle.validate_control_frame(bad_bytes),
            Err(FfiError::Protocol { .. })
        ));
    }
}
