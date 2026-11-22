use keynear_core::RemoteVfsNode;

#[cfg(feature = "fuse")]
use crate::fuse::FuserAdapter;
use crate::fuse::{FuseMountBackend, UnavailableFuseBackend, VfsFuseController};

enum ActiveFuseController {
    Unavailable(VfsFuseController<UnavailableFuseBackend>),
    #[cfg(feature = "fuse")]
    Fuser(VfsFuseController<FuserAdapter>),
}

impl ActiveFuseController {
    fn abort(&mut self) -> bool {
        match self {
            Self::Unavailable(controller) => controller.abort(),
            #[cfg(feature = "fuse")]
            Self::Fuser(controller) => controller.abort(),
        }
    }
}

pub struct FuseProbeGuard {
    controller: ActiveFuseController,
}

impl Drop for FuseProbeGuard {
    fn drop(&mut self) {
        let restored = self.controller.abort();
        tracing::info!(event = "fuse_mount_shutdown", restored);
    }
}

pub fn initialize() -> FuseProbeGuard {
    let root = probe_root();
    #[cfg(feature = "fuse")]
    let controller = if fuse_requested() {
        fuser_controller(&root)
    } else {
        unavailable_controller(&root)
    };
    #[cfg(not(feature = "fuse"))]
    let controller = unavailable_controller(&root);
    FuseProbeGuard { controller }
}

#[cfg(feature = "fuse")]
fn fuse_requested() -> bool {
    std::env::var("KEYNEAR_FUSE")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false)
}

fn unavailable_controller(root: &RemoteVfsNode) -> ActiveFuseController {
    let backend = UnavailableFuseBackend;
    log_availability(backend.available());
    let mut controller = VfsFuseController::new(backend);
    controller.start(root);
    log_state(&controller);
    ActiveFuseController::Unavailable(controller)
}

#[cfg(feature = "fuse")]
fn fuser_controller(root: &RemoteVfsNode) -> ActiveFuseController {
    let mountpoint = std::env::temp_dir().join("keynear-vfs");
    if let Err(err) = std::fs::create_dir_all(&mountpoint) {
        tracing::warn!(event = "fuse_mountpoint_unavailable", error = %err);
        return unavailable_controller(root);
    }
    let adapter = FuserAdapter::new(mountpoint);
    log_availability(adapter.available());
    let mut controller = VfsFuseController::new(adapter);
    controller.start(root);
    log_state(&controller);
    ActiveFuseController::Fuser(controller)
}

fn log_availability(available: bool) {
    tracing::info!(event = "fuse_mount_probe", available);
}

fn log_state<B: FuseMountBackend>(controller: &VfsFuseController<B>) {
    tracing::info!(event = "fuse_mount_state", state = ?controller.state());
}

fn probe_root() -> RemoteVfsNode {
    RemoteVfsNode::root("keynear", "Keynear").expect("static root is valid")
}
