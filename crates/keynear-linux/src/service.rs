use std::future::Future;

use keynear_core::{DaemonConfig, KeynearError};
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

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
        info!(
            event = "daemon_started",
            visibility = ?config.visibility_mode,
            incoming_dir = %config.incoming_directory
        );

        // TODO: Start BLE discovery, QUIC transport, StatusNotifierItem tray, clipboard sync, and audio handoff supervisors.
        shutdown.await;

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

    use tokio::sync::oneshot;

    use super::*;

    #[tokio::test]
    async fn shutdown_loop_completes_once() {
        let config = DaemonConfig::default();
        let (tx, rx) = oneshot::channel::<()>();
        let completions = Arc::new(AtomicUsize::new(0));
        let completions_clone = Arc::clone(&completions);

        let task = tokio::spawn(async move {
            let result = DaemonRuntime::run_with_shutdown(&config, async move {
                let _ = rx.await;
            })
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
}
