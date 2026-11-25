use std::future::Future;

use keynear_core::KeynearError;
use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn lock_session(&self, session_id: &str) -> zbus::Result<()>;
    fn lock_sessions(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/self"
)]
trait Login1Session {
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

fn lock_error(err: impl std::fmt::Display) -> KeynearError {
    KeynearError::Protocol(format!("login1 session lock failed: {err}"))
}

pub trait SessionLock: Send + Sync {
    fn lock_session(
        &self,
        session_id: &str,
    ) -> impl Future<Output = Result<(), KeynearError>> + Send;
    fn lock_all_sessions(&self) -> impl Future<Output = Result<(), KeynearError>> + Send;
    fn is_locked(&self) -> impl Future<Output = Result<bool, KeynearError>> + Send;
}

pub struct Login1SessionLock;

impl SessionLock for Login1SessionLock {
    async fn lock_session(&self, session_id: &str) -> Result<(), KeynearError> {
        let connection = zbus::Connection::system().await.map_err(lock_error)?;
        let proxy = Login1ManagerProxy::new(&connection)
            .await
            .map_err(lock_error)?;
        proxy.lock_session(session_id).await.map_err(lock_error)
    }

    async fn lock_all_sessions(&self) -> Result<(), KeynearError> {
        let connection = zbus::Connection::system().await.map_err(lock_error)?;
        let proxy = Login1ManagerProxy::new(&connection)
            .await
            .map_err(lock_error)?;
        proxy.lock_sessions().await.map_err(lock_error)
    }

    async fn is_locked(&self) -> Result<bool, KeynearError> {
        let connection = zbus::Connection::system().await.map_err(lock_error)?;
        let proxy = Login1SessionProxy::new(&connection)
            .await
            .map_err(lock_error)?;
        proxy.locked_hint().await.map_err(lock_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLock {
        locked: std::sync::atomic::AtomicBool,
        calls: std::sync::atomic::AtomicUsize,
        fail: bool,
    }

    impl SessionLock for FakeLock {
        async fn lock_session(&self, _session_id: &str) -> Result<(), KeynearError> {
            if self.fail {
                return Err(KeynearError::Protocol("denied".to_string()));
            }
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

    #[tokio::test]
    async fn fake_lock_reports_transition_and_failure() {
        let lock = FakeLock {
            locked: std::sync::atomic::AtomicBool::new(false),
            calls: std::sync::atomic::AtomicUsize::new(0),
            fail: false,
        };
        assert!(!lock.is_locked().await.expect("unlocked"));
        lock.lock_session("2").await.expect("lock");
        assert!(lock.is_locked().await.expect("locked"));
        assert_eq!(lock.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let failing = FakeLock {
            locked: std::sync::atomic::AtomicBool::new(false),
            calls: std::sync::atomic::AtomicUsize::new(0),
            fail: true,
        };
        assert!(failing.lock_session("2").await.is_err());
        assert!(!failing.is_locked().await.expect("still unlocked"));
    }
}
