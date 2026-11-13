use keynear_core::{DaemonConfig, KeynearError};
use tracing::info;

pub struct DaemonRuntime;

impl DaemonRuntime {
    pub async fn start(config: &DaemonConfig) -> Result<(), KeynearError> {
        config.validate()?;
        info!(
            visibility = ?config.visibility_mode,
            incoming_dir = %config.incoming_directory,
            "keynear-linux daemon runtime initialized"
        );
        // TODO: Start BLE discovery, QUIC transport, StatusNotifierItem tray, clipboard sync, and audio handoff supervisors.
        Ok(())
    }
}
