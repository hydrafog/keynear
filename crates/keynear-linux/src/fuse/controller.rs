use keynear_core::{read_plan, ReadPlan, RemoteVfsNode};

use super::backend::{FuseMountBackend, MountOutcome};

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

    pub fn start(&mut self, root: &RemoteVfsNode) -> MountOutcome {
        let outcome = self.backend.mount(root);
        self.state = match outcome {
            MountOutcome::Mounted => ControllerState::Mounted,
            MountOutcome::Unavailable => ControllerState::Unavailable,
        };
        outcome
    }

    pub fn abort(&mut self) -> bool {
        let restored = self.backend.unmount();
        self.state = ControllerState::Idle;
        restored
    }

    pub fn plan_read(
        &self,
        file: &RemoteVfsNode,
        offset: u64,
        max_bytes: usize,
    ) -> Option<ReadPlan> {
        if self.state != ControllerState::Mounted {
            return None;
        }
        read_plan(file.size_bytes, offset, max_bytes).ok()
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
        fn availability(&self) -> super::super::backend::MountAvailability {
            if self.mounted {
                super::super::backend::MountAvailability::Available
            } else {
                super::super::backend::MountAvailability::Unavailable
            }
        }

        fn mount(&self, _root: &RemoteVfsNode) -> MountOutcome {
            self.recorder.calls.lock().expect("recorder").push("mount");
            if self.mounted {
                MountOutcome::Mounted
            } else {
                MountOutcome::Unavailable
            }
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
        assert_eq!(controller.start(&root), MountOutcome::Unavailable);
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
        assert_eq!(controller.start(&root), MountOutcome::Mounted);
        assert!(controller.abort());
        assert_eq!(controller.state(), ControllerState::Idle);
        assert_eq!(
            recorder.calls.lock().expect("recorder").as_slice(),
            ["mount", "unmount"]
        );
    }

    #[test]
    fn read_plan_requires_mounted_state() {
        let recorder = Arc::new(Recorder::default());
        let backend = FakeBackend {
            recorder: Arc::clone(&recorder),
            mounted: true,
        };
        let controller = VfsFuseController::new(backend);
        let file = RemoteVfsNode::child("phone-1", "report.txt", false, 5_000).expect("file");
        assert!(controller.plan_read(&file, 0, 1_024).is_none());
    }
}
