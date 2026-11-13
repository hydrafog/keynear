mod service;

use keynear_core::{default_config_path, DaemonConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = default_config_path();
    let config = DaemonConfig::load_or_default(&config_path)?;
    service::DaemonRuntime::start(&config).await?;
    Ok(())
}
