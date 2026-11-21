use keynear_core::RemoteVfsNode;

use crate::fuse::{FuseMountBackend, UnavailableFuseBackend, VfsFuseController};

pub struct FuseProbeGuard {
    controller: VfsFuseController<UnavailableFuseBackend>,
}

impl Drop for FuseProbeGuard {
    fn drop(&mut self) {
        let restored = self.controller.abort();
        tracing::info!(event = "perseus_fuse_shutdown", restored);
    }
}

pub fn initialize() -> FuseProbeGuard {
    let backend = UnavailableFuseBackend;
    let availability = backend.availability();
    tracing::info!(event = "perseus_fuse_probe", available = ?availability);

    let root = RemoteVfsNode::root("keynear", "Keynear").expect("static root is valid");
    let mut controller = VfsFuseController::new(backend);
    let outcome = controller.start(&root);
    tracing::info!(event = "perseus_fuse_mount", outcome = ?outcome);
    FuseProbeGuard { controller }
}
