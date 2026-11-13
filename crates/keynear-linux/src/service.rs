use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{
    default_config_dir, DaemonConfig, KeynearError, Oo7SecretStore, PairedDeviceStore, SecretStore,
    VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;
use tracing::{info, warn};

use crate::ble::{BleDriverParams, BlePeerDriver, DEFAULT_CAPABILITY_FLAGS};

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub local_device_id: String,
    pub display_name: String,
    pub everyone_nearby_timeout: Duration,
    pub enable_bluez: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            local_device_id: default_local_device_id(),
            display_name: default_display_name(),
            everyone_nearby_timeout: Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS),
            enable_bluez: true,
        }
    }
}

fn default_local_device_id() -> String {
    let raw = std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .unwrap_or_else(|_| "keynear-linux".to_string());
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    if cleaned.is_empty() {
        "keynear-linux".to_string()
    } else {
        cleaned
    }
}

fn default_display_name() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Keynear Linux".to_string())
}

pub struct DaemonRuntime;

impl DaemonRuntime {
    pub async fn run_with_shutdown<F>(
        config: &DaemonConfig,
        shutdown: F,
    ) -> Result<(), KeynearError>
    where
        F: Future<Output = ()>,
    {
        config.validate()?;
        let device_store = PairedDeviceStore::default_location();
        let secret_store: Option<Arc<dyn SecretStore>> = match Oo7SecretStore::open_default() {
            Ok(store) => {
                if store.is_file_fallback() {
                    info!(
                        event = "keyring_file_fallback_active",
                        config_dir = %default_config_dir().display()
                    );
                }
                Some(Arc::new(store))
            }
            Err(err) => {
                warn!(event = "keyring_unavailable_degraded", error = %err);
                None
            }
        };

        Self::run_with_deps(
            config,
            RuntimeOptions::default(),
            device_store,
            secret_store,
            shutdown,
        )
        .await
    }

    pub async fn run_with_deps<F>(
        config: &DaemonConfig,
        options: RuntimeOptions,
        device_store: PairedDeviceStore,
        secret_store: Option<Arc<dyn SecretStore>>,
        shutdown: F,
    ) -> Result<(), KeynearError>
    where
        F: Future<Output = ()>,
    {
        config.validate()?;
        let (visibility_tx, visibility_rx) = watch::channel(config.visibility_mode);
        let initial_mode = config.visibility_mode;
        info!(
            event = "daemon_started",
            visibility = ?initial_mode,
            incoming_dir = %config.incoming_directory
        );

        let ble_task = if options.enable_bluez {
            secret_store.as_ref().map(|secrets| {
                let local_id = options.local_device_id.clone();
                let display = options.display_name.clone();
                let rx = visibility_rx.clone();
                let secrets = Arc::clone(secrets);
                let store = device_store.clone();
                tokio::spawn(async move {
                    if let Err(err) = BlePeerDriver::run_bluez(BleDriverParams {
                        local_device_id: local_id,
                        display_name: display,
                        capability_flags: DEFAULT_CAPABILITY_FLAGS,
                        visibility_rx: rx,
                        secret_store: secrets,
                        device_store: store,
                        discovered_tx: None,
                        pairing_decision_rx: None,
                        pairing_event_tx: None,
                    })
                    .await
                    {
                        warn!(event = "ble_unavailable_degraded", error = %err);
                    }
                })
            })
        } else {
            None
        };

        let revert_timer = async {
            if initial_mode == VisibilityMode::EveryoneNearby {
                tokio::time::sleep(options.everyone_nearby_timeout).await;
                let _ = visibility_tx.send(VisibilityMode::PairedOnly);
                info!(event = "visibility_auto_reverted", to = ?VisibilityMode::PairedOnly);
                std::future::pending::<()>().await;
            } else {
                std::future::pending::<()>().await;
            }
        };

        let shutdown_observer = shutdown;
        tokio::select! {
            _ = shutdown_observer => {}
            _ = revert_timer => {}
        }

        drop(visibility_tx);
        if let Some(handle) = ble_task {
            handle.abort();
            let _ = handle.await;
        }

        info!(event = "daemon_stopped");
        Ok(())
    }

    pub async fn run_until_shutdown(config: &DaemonConfig) -> Result<(), KeynearError> {
        let mut sigterm = signal(SignalKind::terminate())?;
        let shutdown = async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
        };
        Self::run_with_shutdown(config, shutdown).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use keynear_core::InMemorySecretStore;
    use tokio::sync::oneshot;

    use super::*;

    #[tokio::test]
    async fn shutdown_loop_completes_once() {
        let config = DaemonConfig::default();
        let (tx, rx) = oneshot::channel::<()>();
        let completions = Arc::new(AtomicUsize::new(0));
        let completions_clone = Arc::clone(&completions);
        let options = RuntimeOptions {
            enable_bluez: false,
            ..Default::default()
        };

        let task = tokio::spawn(async move {
            let result = DaemonRuntime::run_with_deps(
                &config,
                options,
                PairedDeviceStore::default_location(),
                Some(Arc::new(InMemorySecretStore::new())),
                async move {
                    let _ = rx.await;
                },
            )
            .await;
            completions_clone.fetch_add(1, Ordering::SeqCst);
            result
        });

        tx.send(()).expect("send shutdown signal");
        task.await
            .expect("join runtime task")
            .expect("runtime exits cleanly");
        assert_eq!(completions.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn invalid_config_fails_before_awaiting_shutdown() {
        let config = DaemonConfig {
            incoming_directory: String::new(),
            ..Default::default()
        };
        let err = DaemonRuntime::run_with_shutdown(&config, std::future::pending::<()>())
            .await
            .expect_err("invalid config must fail immediately");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "incomingDirectory",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn degrades_gracefully_when_bluetooth_and_keyring_absent() {
        let config = DaemonConfig {
            visibility_mode: VisibilityMode::EveryoneNearby,
            ..Default::default()
        };
        let (tx, rx) = oneshot::channel::<()>();
        let options = RuntimeOptions {
            everyone_nearby_timeout: Duration::from_millis(25),
            enable_bluez: true,
            ..Default::default()
        };

        let task = tokio::spawn(async move {
            DaemonRuntime::run_with_deps(
                &config,
                options,
                PairedDeviceStore::default_location(),
                None,
                async move {
                    let _ = rx.await;
                },
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(45)).await;
        tx.send(()).expect("shutdown signal");
        task.await
            .expect("join task")
            .expect("daemon degrades cleanly without bluetooth or keyring");
    }
}
