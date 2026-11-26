use std::future::Future;
use std::os::unix::fs::MetadataExt;

use keynear_core::KeynearError;
use zbus::{proxy, OwnedObjectPath};

type Login1SessionInfo = (String, u32, String, String, OwnedObjectPath);

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn lock_session(&self, session_id: &str) -> zbus::Result<()>;
    fn lock_sessions(&self) -> zbus::Result<()>;
    fn list_sessions(&self) -> zbus::Result<Vec<Login1SessionInfo>>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/self"
)]
trait Login1Session {
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;
    #[zbus(property, name = "Type")]
    fn session_type(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver"
)]
trait ScreenSaver {
    fn lock(&self) -> zbus::Result<()>;
}

fn lock_error(err: impl std::fmt::Display) -> KeynearError {
    KeynearError::Protocol(format!("login1 session lock failed: {err}"))
}

fn current_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

async fn resolve_user_session(
    connection: &zbus::Connection,
    manager: &Login1ManagerProxy<'_>,
) -> Result<Option<(String, OwnedObjectPath)>, KeynearError> {
    let Some(uid) = current_uid() else {
        return Ok(None);
    };
    let sessions = manager.list_sessions().await.map_err(lock_error)?;
    let mut best: Option<(u8, String, OwnedObjectPath)> = None;
    for (id, session_uid, _user, _seat, path) in sessions {
        if session_uid != uid {
            continue;
        }
        let rank = session_priority(connection, &path).await;
        if best.as_ref().is_none_or(|(best_rank, _, _)| rank > *best_rank) {
            best = Some((rank, id, path));
        }
    }
    Ok(best.map(|(_, id, path)| (id, path)))
}

async fn session_priority(connection: &zbus::Connection, path: &OwnedObjectPath) -> u8 {
    let builder = Login1SessionProxy::builder(connection).path(path.as_ref());
    let Ok(proxy) = builder.build().await else {
        return 1;
    };
    let is_active = proxy.state().await.is_ok_and(|s| s == "active");
    let is_graphical = proxy
        .session_type()
        .await
        .is_ok_and(|t| t == "wayland" || t == "x11");
    match (is_active, is_graphical) {
        (true, true) => 4,
        (true, false) => 3,
        (false, true) => 2,
        (false, false) => 1,
    }
}

async fn lock_via_screensaver() -> Result<(), KeynearError> {
    let session_bus = zbus::Connection::session().await.map_err(lock_error)?;
    let proxy = ScreenSaverProxy::new(&session_bus)
        .await
        .map_err(lock_error)?;
    proxy.lock().await.map_err(lock_error)
}

async fn query_resolved_locked_hint(connection: &zbus::Connection) -> Option<bool> {
    let manager = Login1ManagerProxy::new(connection).await.ok()?;
    let (_, path) = resolve_user_session(connection, &manager).await.ok()??;
    let proxy = Login1SessionProxy::builder(connection)
        .path(path.as_ref())
        .build()
        .await
        .ok()?;
    proxy.locked_hint().await.ok()
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
        if session_id.trim().is_empty() {
            return self.lock_all_sessions().await;
        }
        let login1_result = async {
            let connection = zbus::Connection::system().await.map_err(lock_error)?;
            let proxy = Login1ManagerProxy::new(&connection)
                .await
                .map_err(lock_error)?;
            proxy.lock_session(session_id).await.map_err(lock_error)
        }
        .await;
        if login1_result.is_ok() {
            return Ok(());
        }
        lock_via_screensaver().await.or(login1_result)
    }

    async fn lock_all_sessions(&self) -> Result<(), KeynearError> {
        let login1_result = async {
            let connection = zbus::Connection::system().await.map_err(lock_error)?;
            let proxy = Login1ManagerProxy::new(&connection)
                .await
                .map_err(lock_error)?;
            if let Some((session_id, _)) = resolve_user_session(&connection, &proxy).await? {
                proxy.lock_session(&session_id).await.map_err(lock_error)
            } else {
                proxy.lock_sessions().await.map_err(lock_error)
            }
        }
        .await;
        if login1_result.is_ok() {
            return Ok(());
        }
        lock_via_screensaver().await.or(login1_result)
    }

    async fn is_locked(&self) -> Result<bool, KeynearError> {
        let connection = zbus::Connection::system().await.map_err(lock_error)?;
        if let Some(hint) = query_resolved_locked_hint(&connection).await {
            return Ok(hint);
        }
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
