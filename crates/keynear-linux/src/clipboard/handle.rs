use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use keynear_core::{is_image_mime, KeynearError, MAX_CLIPBOARD_IMAGE_BYTES};
use tokio::sync::{mpsc, oneshot};

use super::tracker::SelfWriteTracker;
use super::ClipboardBackendKind;

pub(crate) enum BackendCmd {
    WriteClipboard {
        mime_type: String,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), KeynearError>>,
    },
    RenderOffer {
        offer_seq: u64,
        mime_type: String,
        reply: std_mpsc::Sender<Result<Vec<u8>, KeynearError>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct ClipboardMonitorHandle {
    pub(crate) backend: ClipboardBackendKind,
    pub(crate) cmd_tx: Option<mpsc::UnboundedSender<BackendCmd>>,
    pub(crate) self_writes: Arc<Mutex<SelfWriteTracker>>,
}

impl ClipboardMonitorHandle {
    pub fn headless() -> Self {
        Self {
            backend: ClipboardBackendKind::HeadlessUnavailable,
            cmd_tx: None,
            self_writes: Arc::new(Mutex::new(SelfWriteTracker::default())),
        }
    }

    pub fn backend(&self) -> ClipboardBackendKind {
        self.backend
    }

    #[cfg(test)]
    pub fn write_clipboard(&self, mime_type: &str, data: &[u8]) -> Result<(), KeynearError> {
        let clean_mime = mime_type.trim();
        if clean_mime.is_empty() {
            return Err(KeynearError::Validation {
                field: "mimeType",
                message: "clipboard MIME type must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }
        {
            let mut tracker = self
                .self_writes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            tracker.record_write(data, now);
            if is_image_mime(clean_mime) {
                tracker.record_image_write(now);
            }
        }
        let Some(ref tx) = self.cmd_tx else {
            return Ok(());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(BackendCmd::WriteClipboard {
            mime_type: clean_mime.to_string(),
            data: data.to_vec(),
            reply: reply_tx,
        })
        .map_err(|_| KeynearError::Transport("clipboard monitor stopped".to_string()))?;
        let _ = reply_rx.blocking_recv();
        Ok(())
    }

    pub async fn write_clipboard_async(
        &self,
        mime_type: &str,
        data: &[u8],
    ) -> Result<(), KeynearError> {
        let clean_mime = mime_type.trim();
        if clean_mime.is_empty() {
            return Err(KeynearError::Validation {
                field: "mimeType",
                message: "clipboard MIME type must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }
        {
            let mut tracker = self
                .self_writes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            tracker.record_write(data, now);
            if is_image_mime(clean_mime) {
                tracker.record_image_write(now);
            }
        }
        let Some(ref tx) = self.cmd_tx else {
            return Ok(());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(BackendCmd::WriteClipboard {
            mime_type: clean_mime.to_string(),
            data: data.to_vec(),
            reply: reply_tx,
        })
        .map_err(|_| KeynearError::Transport("clipboard monitor stopped".to_string()))?;
        reply_rx
            .await
            .map_err(|_| KeynearError::Transport("clipboard write reply dropped".to_string()))?
    }

    pub fn shutdown(&self) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(BackendCmd::Shutdown);
        }
    }
}
