use keynear_core::{
    GatedOperation, KeynearError, PresenceGate, ProximityConfig, ProximityEvaluator,
    ProximitySnapshot, ProximityStatus, RangingSample,
};
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::lock::SessionLock;

pub struct ProximitySupervisor<L: SessionLock> {
    evaluator: ProximityEvaluator,
    lock: L,
    session_id: String,
    lock_enabled: bool,
}

impl<L: SessionLock> ProximitySupervisor<L> {
    pub fn new(
        lock: L,
        config: ProximityConfig,
        session_id: String,
        lock_enabled: bool,
    ) -> Result<Self, KeynearError> {
        Ok(Self {
            evaluator: ProximityEvaluator::new(config)?,
            lock,
            session_id,
            lock_enabled,
        })
    }

    pub fn snapshot(&self) -> ProximitySnapshot {
        self.evaluator.snapshot()
    }

    pub fn allows(&self, operation: GatedOperation) -> bool {
        PresenceGate.allows(operation, self.evaluator.snapshot())
    }

    pub fn update_config(
        &mut self,
        config: ProximityConfig,
        lock_enabled: bool,
    ) -> Result<(), KeynearError> {
        self.evaluator.update_config(config)?;
        self.lock_enabled = lock_enabled;
        Ok(())
    }

    pub async fn observe(
        &mut self,
        sample: RangingSample,
    ) -> Result<ProximitySnapshot, KeynearError> {
        self.evaluator.update(sample)?;
        self.refresh_lock_hint().await;
        if self.should_lock() {
            self.request_lock().await;
        }
        Ok(self.evaluator.snapshot())
    }

    async fn refresh_lock_hint(&mut self) {
        match self.lock.is_locked().await {
            Ok(locked) => self.evaluator.set_session_locked(locked),
            Err(err) => warn!(event = "phoenix_locked_hint_unavailable", error = %err),
        }
    }

    fn should_lock(&self) -> bool {
        let snapshot = self.evaluator.snapshot();
        self.lock_enabled
            && snapshot.status == ProximityStatus::OutOfRange
            && !snapshot.session_locked
    }

    async fn request_lock(&mut self) {
        let result = self.invoke_lock().await;
        self.log_lock_result(result);
    }

    async fn invoke_lock(&self) -> Result<(), KeynearError> {
        if self.session_id.is_empty() {
            self.lock.lock_all_sessions().await
        } else {
            self.lock.lock_session(&self.session_id).await
        }
    }

    fn log_lock_result(&mut self, result: Result<(), KeynearError>) {
        match result {
            Ok(()) => self.mark_locked(),
            Err(err) => log_lock_failure(&err),
        }
    }

    fn mark_locked(&mut self) {
        self.evaluator.set_session_locked(true);
        info!(event = "phoenix_proximity_lock");
    }
}

fn log_lock_failure(err: &KeynearError) {
    warn!(event = "phoenix_proximity_lock_failed", error = %err);
}

pub fn spawn_proximity_loop<L: SessionLock + 'static>(
    supervisor: Arc<Mutex<ProximitySupervisor<L>>>,
    mut rx: mpsc::Receiver<RangingSample>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    tasks.push(tokio::spawn(async move {
        let mut sequence = 0u64;
        while let Some(mut sample) = rx.recv().await {
            sequence += 1;
            sample.sequence = sequence;
            let mut guard = supervisor.lock().await;
            match guard.observe(sample).await {
                Ok(snapshot) => {
                    let blocked = [
                        GatedOperation::ClipboardSync,
                        GatedOperation::OtpForwarding,
                        GatedOperation::AutoAcceptTransfer,
                    ]
                    .into_iter()
                    .filter(|operation| !guard.allows(*operation))
                    .count();
                    let current = guard.snapshot();
                    info!(
                        event = "phoenix_proximity_update",
                        status = ?snapshot.status,
                        distance_meters = ?current.distance_meters,
                        blocked_operations = blocked
                    );
                }
                Err(err) => warn!(event = "phoenix_ranging_rejected", error = %err),
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLock {
        locked: std::sync::atomic::AtomicBool,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl SessionLock for FakeLock {
        async fn lock_session(&self, _session_id: &str) -> Result<(), KeynearError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.locked.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn lock_all_sessions(&self) -> Result<(), KeynearError> {
            self.lock_session("").await
        }

        async fn is_locked(&self) -> Result<bool, KeynearError> {
            Ok(self.locked.load(std::sync::atomic::Ordering::SeqCst))
        }
    }

    fn sample(seq: u64, distance: f32) -> RangingSample {
        RangingSample {
            device_id: "phone-1".to_string(),
            technology: keynear_core::RangingTechnology::Uwb,
            distance_meters: Some(distance),
            rssi_dbm: None,
            sequence: seq,
        }
    }

    #[tokio::test]
    async fn leaving_range_locks_and_blocks_presence_gated_paths() {
        let lock = FakeLock {
            locked: std::sync::atomic::AtomicBool::new(false),
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let mut supervisor =
            ProximitySupervisor::new(lock, ProximityConfig::default(), "2".to_string(), true)
                .expect("supervisor");
        for seq in 1..=2 {
            supervisor.observe(sample(seq, 1.0)).await.expect("near");
        }
        assert!(supervisor.allows(GatedOperation::ClipboardSync));
        for seq in 3..=4 {
            supervisor.observe(sample(seq, 9.0)).await.expect("far");
        }
        assert_eq!(supervisor.snapshot().status, ProximityStatus::OutOfRange);
        assert!(supervisor.snapshot().session_locked);
        assert!(!supervisor.allows(GatedOperation::OtpForwarding));
        assert!(!supervisor.allows(GatedOperation::AutoAcceptTransfer));
    }

    #[tokio::test]
    async fn disabled_lock_does_not_call_login1() {
        let lock = FakeLock {
            locked: std::sync::atomic::AtomicBool::new(false),
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let mut supervisor =
            ProximitySupervisor::new(lock, ProximityConfig::default(), String::new(), false)
                .expect("supervisor");
        for seq in 1..=3 {
            supervisor.observe(sample(seq, 9.0)).await.expect("far");
        }
        assert_eq!(supervisor.snapshot().status, ProximityStatus::OutOfRange);
        assert!(!supervisor.snapshot().session_locked);
    }

    #[tokio::test]
    async fn live_config_update_recalibrates_thresholds_and_enables_lock() {
        let lock = FakeLock {
            locked: std::sync::atomic::AtomicBool::new(false),
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let initial = ProximityConfig::from_desk_distance(10.0);
        let mut supervisor =
            ProximitySupervisor::new(lock, initial, "2".to_string(), false).expect("supervisor");
        for seq in 1..=2 {
            supervisor.observe(sample(seq, 2.0)).await.expect("near");
        }
        assert_eq!(supervisor.snapshot().status, ProximityStatus::InRange);

        let tighter = ProximityConfig::from_desk_distance(1.5);
        assert!(tighter.exit_rssi_dbm > initial.exit_rssi_dbm);
        supervisor
            .update_config(tighter, true)
            .expect("update config");
        for seq in 3..=4 {
            supervisor.observe(sample(seq, 2.5)).await.expect("now far");
        }
        assert_eq!(supervisor.snapshot().status, ProximityStatus::OutOfRange);
        assert!(supervisor.snapshot().session_locked);
    }
}
