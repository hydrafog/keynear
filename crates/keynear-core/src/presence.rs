use crate::proximity::{ProximitySnapshot, ProximityStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatedOperation {
    ClipboardSync,
    OtpForwarding,
    AutoAcceptTransfer,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PresenceGate;

impl PresenceGate {
    pub fn allows(self, operation: GatedOperation, snapshot: ProximitySnapshot) -> bool {
        let _ = operation;
        !snapshot.session_locked && snapshot.status == ProximityStatus::InRange
    }

    pub fn blocked_operations(self, snapshot: ProximitySnapshot) -> Vec<GatedOperation> {
        if self.allows(GatedOperation::ClipboardSync, snapshot) {
            return Vec::new();
        }
        vec![
            GatedOperation::ClipboardSync,
            GatedOperation::OtpForwarding,
            GatedOperation::AutoAcceptTransfer,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(status: ProximityStatus, session_locked: bool) -> ProximitySnapshot {
        ProximitySnapshot {
            status,
            distance_meters: Some(1.0),
            session_locked,
        }
    }

    #[test]
    fn in_range_unlocked_allows_every_operation() {
        let gate = PresenceGate;
        let current = snapshot(ProximityStatus::InRange, false);
        for operation in [
            GatedOperation::ClipboardSync,
            GatedOperation::OtpForwarding,
            GatedOperation::AutoAcceptTransfer,
        ] {
            assert!(gate.allows(operation, current));
        }
        assert!(gate.blocked_operations(current).is_empty());
    }

    #[test]
    fn out_of_range_pauses_all_three_paths() {
        let gate = PresenceGate;
        let away = snapshot(ProximityStatus::OutOfRange, false);
        assert!(!gate.allows(GatedOperation::ClipboardSync, away));
        assert!(!gate.allows(GatedOperation::OtpForwarding, away));
        assert!(!gate.allows(GatedOperation::AutoAcceptTransfer, away));
        assert_eq!(gate.blocked_operations(away).len(), 3);
    }

    #[test]
    fn locked_session_blocks_continuity_even_when_nearby() {
        let gate = PresenceGate;
        let locked = snapshot(ProximityStatus::InRange, true);
        assert!(!gate.allows(GatedOperation::OtpForwarding, locked));
    }

    #[test]
    fn unknown_presence_fails_closed_and_resumes_on_return() {
        let gate = PresenceGate;
        assert!(!gate.allows(
            GatedOperation::ClipboardSync,
            snapshot(ProximityStatus::Unknown, false)
        ));
        let returned = snapshot(ProximityStatus::InRange, false);
        assert!(gate.allows(GatedOperation::ClipboardSync, returned));
    }
}
