use crate::model::DaemonConfig;
use crate::proto::MAX_CONTROL_FRAME_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreStatus {
    pub version: String,
    pub max_control_frame_bytes: u32,
    pub ready: bool,
}

#[derive(Debug, Default)]
pub struct CoreHandle;

impl CoreHandle {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> CoreStatus {
        get_core_status()
    }
}

pub fn get_core_status() -> CoreStatus {
    CoreStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        max_control_frame_bytes: MAX_CONTROL_FRAME_BYTES as u32,
        ready: true,
    }
}

pub fn default_config_summary() -> String {
    let config = DaemonConfig::default();
    format!(
        "visibility={:?},incoming_dir={}",
        config.visibility_mode, config.incoming_directory
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_status_reports_ready() {
        let handle = CoreHandle::new();
        let status = handle.status();
        assert!(status.ready);
        assert_eq!(
            status.max_control_frame_bytes,
            MAX_CONTROL_FRAME_BYTES as u32
        );
    }
}
