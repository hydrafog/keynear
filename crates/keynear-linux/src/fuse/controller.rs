use keynear_core::RemoteVfsNode;

use super::backend::FuseMountBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerState {
    Idle,
    Mounted,
    Unavailable,
}

pub struct VfsFuseController<B: FuseMountBackend> {
    backend: B,
    state: ControllerState,
}

impl<B: FuseMountBackend> VfsFuseController<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: ControllerState::Idle,
        }
    }

    pub fn start(&mut self, root: &RemoteVfsNode) -> bool {
        let mounted = self.backend.mount(root);
        self.state = if mounted {
            ControllerState::Mounted
        } else {
            ControllerState::Unavailable
        };
        mounted
    }

    pub fn abort(&mut self) -> bool {
        let restored = self.backend.unmount();
        self.state = ControllerState::Idle;
        restored
    }

    pub fn state(&self) -> ControllerState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<&'static str>>,
    }

    struct FakeBackend {
        recorder: Arc<Recorder>,
        mounted: bool,
    }

    impl FuseMountBackend for FakeBackend {
        fn available(&self) -> bool {
            self.mounted
        }

        fn mount(&self, _root: &RemoteVfsNode) -> bool {
            self.recorder.calls.lock().expect("recorder").push("mount");
            self.mounted
        }

        fn unmount(&self) -> bool {
            self.recorder
                .calls
                .lock()
                .expect("recorder")
                .push("unmount");
            true
        }
    }

    #[test]
    fn fuse_backend_unavailable_never_propagates() {
        let recorder = Arc::new(Recorder::default());
        let backend = FakeBackend {
            recorder: Arc::clone(&recorder),
            mounted: false,
        };
        let mut controller = VfsFuseController::new(backend);
        let root = RemoteVfsNode::root("phone-1", "Phone").expect("root");
        assert!(!controller.start(&root));
        assert_eq!(controller.state(), ControllerState::Unavailable);
        assert_eq!(
            recorder.calls.lock().expect("recorder").as_slice(),
            ["mount"]
        );
    }

    #[test]
    fn aborted_mount_restores_state() {
        let recorder = Arc::new(Recorder::default());
        let backend = FakeBackend {
            recorder: Arc::clone(&recorder),
            mounted: true,
        };
        let mut controller = VfsFuseController::new(backend);
        let root = RemoteVfsNode::root("phone-1", "Phone").expect("root");
        assert!(controller.start(&root));
        assert!(controller.abort());
        assert_eq!(controller.state(), ControllerState::Idle);
        assert_eq!(
            recorder.calls.lock().expect("recorder").as_slice(),
            ["mount", "unmount"]
        );
    }
}
