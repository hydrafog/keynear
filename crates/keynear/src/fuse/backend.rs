use keynear_core::RemoteVfsNode;

pub trait FuseMountBackend: Send {
    fn available(&self) -> bool;
    fn mount(&self, root: &RemoteVfsNode) -> bool;
    fn unmount(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct UnavailableFuseBackend;

impl FuseMountBackend for UnavailableFuseBackend {
    fn available(&self) -> bool {
        false
    }

    fn mount(&self, root: &RemoteVfsNode) -> bool {
        tracing::warn!(
            event = "fuse_backend_unavailable",
            root = %root.document_id,
            reason = "libfuse3 or kernel FUSE is unavailable"
        );
        false
    }

    fn unmount(&self) -> bool {
        true
    }
}
