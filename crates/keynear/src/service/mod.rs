pub(crate) mod audio_control;
pub mod clipboard;
pub(crate) mod clipboard_bridge;
pub(crate) mod outbound;
pub mod pairing;
pub mod runtime;
pub mod transfer;

pub use pairing::{apply_discovered_lan_endpoint, sync_paired_devices_and_policies};
pub use runtime::DaemonRuntime;
pub use runtime::RuntimeDeps;
pub use runtime::RuntimeOptions;

#[cfg(test)]
mod tests;
