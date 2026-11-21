use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::config::{CLIPBOARD_ECHO_WINDOW_SECS, MAX_SELF_WRITE_HISTORY};

#[derive(Debug, Default)]
pub struct SelfWriteTracker {
    entries: VecDeque<([u8; 32], Instant)>,
    last_image_write: Option<Instant>,
}

impl SelfWriteTracker {
    pub fn record_write(&mut self, data: &[u8], now: Instant) {
        self.prune(now);
        if self.entries.len() >= MAX_SELF_WRITE_HISTORY {
            self.entries.pop_front();
        }
        let digest: [u8; 32] = Sha256::digest(data).into();
        self.entries.push_back((digest, now));
    }

    pub fn record_image_write(&mut self, now: Instant) {
        self.prune(now);
        self.last_image_write = Some(now);
    }

    pub fn is_recent_image_write(&mut self, now: Instant) -> bool {
        self.prune(now);
        self.last_image_write.is_some()
    }

    pub fn is_recent_echo(&mut self, data: &[u8], now: Instant) -> bool {
        self.prune(now);
        let digest: [u8; 32] = Sha256::digest(data).into();
        self.entries.iter().any(|(d, _)| *d == digest)
    }

    fn prune(&mut self, now: Instant) {
        let window = Duration::from_secs(CLIPBOARD_ECHO_WINDOW_SECS);
        while let Some((_, ts)) = self.entries.front() {
            if now.saturating_duration_since(*ts) >= window {
                self.entries.pop_front();
            } else {
                break;
            }
        }
        if let Some(ts) = self.last_image_write {
            if now.saturating_duration_since(ts) >= window {
                self.last_image_write = None;
            }
        }
    }
}
