use std::time::Duration;

pub const CLIPBOARD_ECHO_WINDOW_SECS: u64 = 5;
pub(crate) const MAX_SELF_WRITE_HISTORY: usize = 16;
pub(crate) const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const X11_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(crate) const X11_MAX_TARGET_ATOMS: usize = 128;
