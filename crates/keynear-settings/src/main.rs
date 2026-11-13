use keynear_core::{default_config_path, DaemonConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = default_config_path();
    let config = DaemonConfig::load_or_default(&config_path)?;
    info!(
        visibility = ?config.visibility_mode,
        incoming_dir = %config.incoming_directory,
        "keynear-settings configuration loaded"
    );
    // TODO: Launch on-demand GTK4/relm4 SimpleComponent preferences window.
    Ok(())
}
