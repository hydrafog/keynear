mod backend;
mod config;
mod handle;
mod mime;
mod tracker;
mod wayland;
mod x11;

#[cfg(test)]
mod tests;

pub use handle::ClipboardMonitorHandle;

#[cfg(test)]
pub use backend::{probe_clipboard_backend, select_backend_from_caps, ProbeCapabilities};

use keynear_core::DelayedRenderCallback;

pub use backend::start_clipboard_monitor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardBackendKind {
    ExtDataControlV1,
    ZwlrDataControlV1,
    X11,
    HeadlessUnavailable,
}

impl ClipboardBackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExtDataControlV1 => "ext-data-control-v1",
            Self::ZwlrDataControlV1 => "zwlr-data-control-v1",
            Self::X11 => "x11",
            Self::HeadlessUnavailable => "headless-unavailable",
        }
    }
}

#[derive(Clone)]
pub enum ClipboardChange {
    Text {
        mime_type: String,
        text: String,
    },
    ImageOffer {
        offered_mimes: Vec<String>,
        render: DelayedRenderCallback,
    },
}
