#![allow(clippy::cognitive_complexity)]

use super::*;

fn meta(device_id: &str, display_name: &str, role: DeviceRole) -> PairingPeerMetadata {
    PairingPeerMetadata {
        device_id: device_id.to_string(),
        display_name: display_name.to_string(),
        device_role: role,
        capability_flags: 0x07,
    }
}

fn initiator(timeout: Duration) -> NoisePairingSession {
    NoisePairingSession::new_with_timeout(
        PairingRole::Initiator,
        &[0x11; 32],
        meta("phone-1", "Pixel", DeviceRole::Mobile),
        timeout,
    )
    .expect("initiator")
}

#[test]
fn new_with_timeout_records_role_deadline_and_remaining_time() {
    let session = initiator(Duration::from_secs(30));
    assert_eq!(session.role(), PairingRole::Initiator);
    assert_eq!(session.state(), PairingState::HandshakeM1);
    assert!(session.remaining_secs() <= 30);
    assert!(session.remaining_secs() > 0);
    assert_eq!(
        session.remaining_secs_at(Instant::now() + Duration::from_secs(60)),
        0
    );
    assert!(session.peer_static_pubkey().is_none());
}

#[test]
fn start_handshake_at_enforces_expiry_and_initiator_role() {
    let mut expired = initiator(Duration::ZERO);
    let err = expired
        .start_handshake_at(Instant::now() + Duration::from_secs(1))
        .expect_err("expired session");
    assert!(matches!(err, KeynearError::PairingExpired));

    let mut responder = NoisePairingSession::new_with_timeout(
        PairingRole::Responder,
        &[0x22; 32],
        meta("linux-1", "ThinkPad", DeviceRole::Desktop),
        Duration::from_secs(30),
    )
    .expect("responder");
    let err = responder
        .start_handshake_at(Instant::now())
        .expect_err("wrong role");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "pairingState"));
}

#[test]
fn step_at_rejects_expired_and_out_of_bounds_frames() {
    let mut expired = initiator(Duration::ZERO);
    let err = expired
        .step_at(&[0xAA], Instant::now() + Duration::from_secs(1))
        .expect_err("expired frame");
    assert!(matches!(err, KeynearError::PairingExpired));

    let mut fresh = initiator(Duration::from_secs(30));
    let err = fresh.step_at(&[], Instant::now()).expect_err("empty frame");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "gattFrame"));
    let err = fresh
        .step_at(&[0u8; MAX_GATT_PAIRING_FRAME_BYTES + 1], Instant::now())
        .expect_err("oversized frame");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "gattFrame"));
}

#[test]
fn ensure_not_expired_at_marks_expired_and_clear_secrets_drops_the_handshake() {
    let mut session = initiator(Duration::from_secs(30));
    let err = session
        .ensure_not_expired_at(Instant::now() + Duration::from_secs(60))
        .expect_err("forced expiry");
    assert!(matches!(err, KeynearError::PairingExpired));
    assert_eq!(session.state(), PairingState::Expired);
    assert!(matches!(
        session.ensure_not_expired_at(Instant::now()),
        Err(KeynearError::PairingExpired)
    ));

    let mut session = initiator(Duration::from_secs(30));
    session.start_handshake().expect("start handshake");
    assert_eq!(session.state(), PairingState::HandshakeM2);
    session.clear_secrets();
    let err = session
        .step_at(&[0xAA], Instant::now())
        .expect_err("handshake state removed");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "pairingState"));
}

#[test]
fn complete_with_peer_export_at_requires_approved_and_unexpired_state() {
    let mut session = initiator(Duration::from_secs(30));
    let err = session
        .complete_with_peer_export_at(&[0xAA], Instant::now())
        .expect_err("wrong state");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "pairingState"));

    let err = session
        .ensure_not_expired_at(Instant::now() + Duration::from_secs(60))
        .expect_err("forced expiry");
    assert!(matches!(err, KeynearError::PairingExpired));
    let err = session
        .complete_with_peer_export_at(&[0xAA], Instant::now())
        .expect_err("expired session");
    assert!(matches!(err, KeynearError::PairingExpired));
}

#[test]
fn metadata_validate_trims_fields_and_rejects_bad_values() {
    let trimmed = meta(" phone-1 ", "  Pixel  ", DeviceRole::Mobile)
        .validate()
        .expect("trimmed metadata");
    assert_eq!(trimmed.device_id, "phone-1");
    assert_eq!(trimmed.display_name, "Pixel");

    let blank_name = meta("phone-1", "   ", DeviceRole::Mobile);
    assert!(
        matches!(blank_name.validate(), Err(KeynearError::Validation { field, .. }) if field == "displayName")
    );
    let long_name = meta(
        "phone-1",
        &"x".repeat(MAX_DISPLAY_NAME_BYTES + 1),
        DeviceRole::Mobile,
    );
    assert!(
        matches!(long_name.validate(), Err(KeynearError::Validation { field, .. }) if field == "displayName")
    );
    let bad_id = meta("../phone", "Pixel", DeviceRole::Mobile);
    assert!(
        matches!(bad_id.validate(), Err(KeynearError::Validation { field, .. }) if field == "deviceId")
    );
    assert!(matches!(
        bad_id.encode(),
        Err(KeynearError::Validation { .. })
    ));
}

#[test]
fn metadata_encode_decode_round_trips_and_rejects_malformed_bytes() {
    let original = meta("phone-1", "Pixel 9", DeviceRole::Mobile);
    let encoded = original.encode().expect("encode");
    assert_eq!(
        PairingPeerMetadata::decode(&encoded).expect("decode"),
        original
    );

    assert!(matches!(
        PairingPeerMetadata::decode(&[0u8; 7]),
        Err(KeynearError::Protocol(message)) if message.contains("too short")
    ));
    let mut bad_version = encoded.clone();
    bad_version[0] = METADATA_FORMAT_VERSION + 1;
    assert!(matches!(
        PairingPeerMetadata::decode(&bad_version),
        Err(KeynearError::Protocol(message)) if message.contains("unsupported pairing metadata version")
    ));
    let mut bad_role = encoded.clone();
    bad_role[1] = 0x99;
    assert!(matches!(
        PairingPeerMetadata::decode(&bad_role),
        Err(KeynearError::Protocol(message)) if message.contains("unknown device role byte")
    ));
    let mut empty_id = encoded.clone();
    empty_id[6] = 0;
    assert!(matches!(
        PairingPeerMetadata::decode(&empty_id),
        Err(KeynearError::Protocol(message)) if message.contains("invalid device_id length")
    ));
    let mut empty_name = encoded;
    let id_len = empty_name[6] as usize;
    empty_name[7 + id_len] = 0;
    assert!(matches!(
        PairingPeerMetadata::decode(&empty_name),
        Err(KeynearError::Protocol(message)) if message.contains("invalid display_name length")
    ));

    let mut invalid_utf8 = vec![METADATA_FORMAT_VERSION, 0x02];
    invalid_utf8.extend_from_slice(&1u32.to_le_bytes());
    invalid_utf8.push(2);
    invalid_utf8.extend_from_slice(&[0xFF, 0xFF]);
    invalid_utf8.push(1);
    invalid_utf8.push(b'X');
    assert!(matches!(
        PairingPeerMetadata::decode(&invalid_utf8),
        Err(KeynearError::Protocol(_))
    ));
}
