use keynear_core::RemoteVfsNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountOutcome {
    Mounted,
    Unavailable,
}

pub trait FuseMountBackend: Send {
    fn availability(&self) -> MountAvailability;
    fn mount(&self, root: &RemoteVfsNode) -> MountOutcome;
    fn unmount(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct UnavailableFuseBackend;

impl FuseMountBackend for UnavailableFuseBackend {
    fn availability(&self) -> MountAvailability {
        MountAvailability::Unavailable
    }

    fn mount(&self, root: &RemoteVfsNode) -> MountOutcome {
        tracing::warn!(
            event = "perseus_fuse_unavailable",
            root = %root.document_id,
            reason = "libfuse3 or kernel FUSE is unavailable"
        );
        MountOutcome::Unavailable
    }

    fn unmount(&self) -> bool {
        true
    }
}
