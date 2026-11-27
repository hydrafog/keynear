use super::*;
use keynear_core::{PairingRole, PairingState};

#[test]
fn ffi_pairing_role_conversion_preserves_variant() {
    assert_eq!(
        FfiPairingRole::from(PairingRole::Initiator),
        FfiPairingRole::Initiator
    );
    assert_eq!(
        FfiPairingRole::from(PairingRole::Responder),
        FfiPairingRole::Responder
    );
}

#[test]
fn ffi_pairing_state_conversion_preserves_all_variants() {
    assert_eq!(
        FfiPairingState::from(PairingState::HandshakeM1),
        FfiPairingState::HandshakeM1
    );
    assert_eq!(
        FfiPairingState::from(PairingState::HandshakeM2),
        FfiPairingState::HandshakeM2
    );
    assert_eq!(
        FfiPairingState::from(PairingState::HandshakeM3),
        FfiPairingState::HandshakeM3
    );
    assert_eq!(
        FfiPairingState::from(PairingState::AwaitingConfirmation),
        FfiPairingState::AwaitingConfirmation
    );
    assert_eq!(
        FfiPairingState::from(PairingState::Approved),
        FfiPairingState::Approved
    );
    assert_eq!(
        FfiPairingState::from(PairingState::Expired),
        FfiPairingState::Expired
    );
    assert_eq!(
        FfiPairingState::from(PairingState::Rejected),
        FfiPairingState::Rejected
    );
    assert_eq!(
        FfiPairingState::from(PairingState::Completed),
        FfiPairingState::Completed
    );
}

#[test]
fn ffi_static_keypair_debug_redacts_private_key() {
    let keypair = FfiStaticKeypair {
        private_key: b"super-secret-private-material".to_vec(),
        public_key_hex: "abcd1234".to_string(),
    };

    let rendered = format!("{keypair:?}");
    assert!(rendered.contains("public_key_hex: \"abcd1234\""));
    assert!(rendered.contains("private_key: \"[REDACTED]\""));
    assert!(!rendered.contains("super-secret-private-material"));
}
