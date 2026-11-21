pub mod clipboard;
pub mod pairing;
pub mod runtime;
pub mod transfer;

#[allow(unused_imports)]
pub use pairing::{apply_discovered_lan_endpoint, sync_paired_devices_and_policies};
pub use runtime::DaemonRuntime;
#[allow(unused_imports)]
pub use runtime::RuntimeDeps;
#[allow(unused_imports)]
pub use runtime::RuntimeOptions;

#[cfg(test)]
mod tests;
