use std::path::PathBuf;
use std::sync::Arc;

use keynear_protocol::{DaemonConfig, VisibilityMode};
use tokio::sync::{watch, Mutex, Notify};
use tokio::time::Instant;
use tracing::info;

use super::RuntimeOptions;

pub(crate) struct RuntimeState {
    pub visibility_tx: watch::Sender<VisibilityMode>,
    pub visibility_rx: watch::Receiver<VisibilityMode>,
    pub config_watch_tx: watch::Sender<DaemonConfig>,
    pub config_watch_rx: watch::Receiver<DaemonConfig>,
    pub config_state: Arc<Mutex<DaemonConfig>>,
    pub config_path: Option<PathBuf>,
    pub everyone_deadline: Arc<Mutex<Option<Instant>>>,
    pub deadline_notify: Arc<Notify>,
    pub refresh_notify: Arc<Notify>,
}

pub(super) fn init_state(config: &DaemonConfig, options: &RuntimeOptions) -> RuntimeState {
    let (visibility_tx, visibility_rx) = watch::channel(config.visibility_mode);
    let (config_watch_tx, config_watch_rx) = watch::channel(config.clone());
    let initial_mode = config.visibility_mode;
    let config_state = Arc::new(Mutex::new(config.clone()));
    let config_path = options.config_path.clone();
    let everyone_deadline = Arc::new(Mutex::new(
        if initial_mode == VisibilityMode::EveryoneNearby {
            Some(Instant::now() + options.everyone_nearby_timeout)
        } else {
            None
        },
    ));
    info!(
        event = "daemon_started",
        visibility = ?initial_mode,
        incoming_dir = %config.incoming_directory
    );
    RuntimeState {
        visibility_tx,
        visibility_rx,
        config_watch_tx,
        config_watch_rx,
        config_state,
        config_path,
        everyone_deadline,
        deadline_notify: Arc::new(Notify::new()),
        refresh_notify: Arc::new(Notify::new()),
    }
}
