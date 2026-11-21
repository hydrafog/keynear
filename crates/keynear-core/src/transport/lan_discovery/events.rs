use super::*;

pub(super) fn record_daemon_event(
    event: &DaemonEvent,
    degraded: &AtomicBool,
    logged_degraded: &AtomicBool,
) -> bool {
    match event {
        DaemonEvent::Error(err) => {
            mark_degraded(degraded, logged_degraded, &err.to_string());
            true
        }
        DaemonEvent::IpAdd(ip) => {
            log_ip_added(ip);
            false
        }
        DaemonEvent::IpDel(ip) => {
            log_ip_removed(ip);
            false
        }
        _ => false,
    }
}

fn log_ip_added(ip: &IpAddr) {
    debug!(event = "mdns_ip_added", %ip);
}

fn log_ip_removed(ip: &IpAddr) {
    debug!(event = "mdns_ip_removed", %ip);
}

pub(super) fn mark_degraded(degraded: &AtomicBool, logged_degraded: &AtomicBool, reason: &str) {
    degraded.store(true, Ordering::Relaxed);
    if !logged_degraded.swap(true, Ordering::AcqRel) {
        warn!(event = "mdns_degraded", reason = %reason);
    }
}
