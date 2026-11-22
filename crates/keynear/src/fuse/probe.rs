use keynear_core::RemoteVfsNode;

use crate::fuse::{FuseMountBackend, UnavailableFuseBackend, VfsFuseController};

pub struct FuseProbeGuard {
    controller: VfsFuseController<UnavailableFuseBackend>,
}

impl Drop for FuseProbeGuard {
    fn drop(&mut self) {
        let restored = self.controller.abort();
        tracing::info!(event = "fuse_mount_shutdown", restored);
    }
}

pub fn initialize() -> FuseProbeGuard {
    let backend = UnavailableFuseBackend;
    log_availability(backend.available());
    let root = probe_root();
    let mut controller = VfsFuseController::new(backend);
    controller.start(&root);
    log_controller_state(&controller);
    FuseProbeGuard { controller }
}

fn log_availability(available: bool) {
    tracing::info!(event = "fuse_mount_probe", available);
}

fn probe_root() -> RemoteVfsNode {
    RemoteVfsNode::root("keynear", "Keynear").expect("static root is valid")
}

fn log_controller_state(controller: &VfsFuseController<UnavailableFuseBackend>) {
    tracing::info!(event = "fuse_mount_state", state = ?controller.state());
}
