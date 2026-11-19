pub mod crypto;
pub mod session;

pub use crypto::{EXPORT_CONTRIBUTION_LEN, MAX_GATT_PAIRING_FRAME_BYTES, PAIRING_PROTOCOL};
pub use session::{
    NoisePairingSession, PairingCompleted, PairingPeerMetadata, PairingRole, PairingState,
};

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use keynear_protocol::{DeviceRole, KeynearError, PAIRING_TIMEOUT_SECS};
    use x25519_dalek::{PublicKey, StaticSecret};

    use super::*;

    fn meta(id: &str, name: &str, role: DeviceRole) -> PairingPeerMetadata {
        PairingPeerMetadata {
            device_id: id.to_string(),
            display_name: name.to_string(),
            device_role: role,
            capability_flags: 0x05,
        }
    }

    #[test]
    fn noise_xx_handshake_and_reconnect_key_match() {
        let init_priv = StaticSecret::random().to_bytes();
        let resp_priv = StaticSecret::random().to_bytes();
        let init_pub = PublicKey::from(&StaticSecret::from(init_priv)).to_bytes();
        let resp_pub = PublicKey::from(&StaticSecret::from(resp_priv)).to_bytes();

        let mut initiator = NoisePairingSession::new_initiator(
            &init_priv,
            meta("android-1", "Pixel 9", DeviceRole::Mobile),
        )
        .expect("init session");
        let mut responder = NoisePairingSession::new_responder(
            &resp_priv,
            meta("linux-1", "ThinkPad", DeviceRole::Desktop),
        )
        .expect("resp session");

        let m1 = initiator.start_handshake().expect("m1");
        let m2 = responder.step(&m1).expect("step m1").expect("m2");
        let m3 = initiator.step(&m2).expect("step m2").expect("m3");
        assert!(responder.step(&m3).expect("step m3").is_none());

        assert_eq!(initiator.state(), PairingState::AwaitingConfirmation);
        assert_eq!(responder.state(), PairingState::AwaitingConfirmation);

        let init_code = initiator.formatted_verification_code().expect("init code");
        let resp_code = responder.formatted_verification_code().expect("resp code");
        assert_eq!(init_code.len(), 7);
        assert_eq!(init_code, resp_code);

        let init_export = initiator.approve_and_build_export().expect("init export");
        let resp_export = responder.approve_and_build_export().expect("resp export");

        let init_done = initiator
            .complete_with_peer_export(&resp_export)
            .expect("init complete");
        let resp_done = responder
            .complete_with_peer_export(&init_export)
            .expect("resp complete");

        assert_eq!(init_done.peer_static_pubkey, resp_pub);
        assert_eq!(resp_done.peer_static_pubkey, init_pub);
        assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);
        assert_eq!(init_done.peer_metadata.device_id, "linux-1");
        assert_eq!(resp_done.peer_metadata.device_id, "android-1");
    }

    #[test]
    fn expired_or_mismatched_pairing_fails_and_zeroizes() {
        let init_priv = StaticSecret::random().to_bytes();
        let resp_priv = StaticSecret::random().to_bytes();

        let mut initiator = NoisePairingSession::new_initiator(
            &init_priv,
            meta("android-1", "Pixel 9", DeviceRole::Mobile),
        )
        .expect("init session");
        let mut responder = NoisePairingSession::new_responder(
            &resp_priv,
            meta("linux-1", "ThinkPad", DeviceRole::Desktop),
        )
        .expect("resp session");

        let m1 = initiator.start_handshake().expect("m1");
        let m2 = responder.step(&m1).expect("m2").unwrap();
        let m3 = initiator.step(&m2).expect("m3").unwrap();
        responder.step(&m3).expect("m3 complete");

        let expired_time = Instant::now() + Duration::from_secs(PAIRING_TIMEOUT_SECS + 1);
        let err = initiator
            .approve_and_build_export_at(expired_time)
            .expect_err("expired approval must fail");
        assert!(matches!(err, KeynearError::PairingExpired));
        assert_eq!(initiator.state(), PairingState::Expired);
        assert!(initiator.verification_code().is_err());
        assert!(initiator.handshake_hash.is_none());
        assert!(initiator.local_contribution.is_none());

        let mut mismatched = responder.approve_and_build_export().expect("resp export");
        mismatched[0] ^= 0xFF;
        let mut other = NoisePairingSession::new_initiator(
            &init_priv,
            meta("android-2", "Pixel 8", DeviceRole::Mobile),
        )
        .expect("other init");
        assert!(other.approve_and_build_export().is_err());
    }

    #[test]
    fn rejects_malformed_gatt_frames_double_approve_and_redacts_logs() {
        let init_priv = StaticSecret::random().to_bytes();
        let resp_priv = StaticSecret::random().to_bytes();

        let mut initiator = NoisePairingSession::new_initiator(
            &init_priv,
            meta("android-1", "Pixel 9", DeviceRole::Mobile),
        )
        .expect("init session");
        let mut responder = NoisePairingSession::new_responder(
            &resp_priv,
            meta("linux-1", "ThinkPad", DeviceRole::Desktop),
        )
        .expect("resp session");

        assert!(matches!(
            responder.step(&[]),
            Err(KeynearError::Validation {
                field: "gattFrame",
                ..
            })
        ));
        let oversized = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES + 1];
        assert!(matches!(
            responder.step(&oversized),
            Err(KeynearError::Validation {
                field: "gattFrame",
                ..
            })
        ));

        let m1 = initiator.start_handshake().expect("m1");
        let m2 = responder.step(&m1).expect("m2").unwrap();
        assert!(matches!(responder.step(&m2), Err(KeynearError::Crypto(_))));

        let mut resp2 = NoisePairingSession::new_responder(
            &resp_priv,
            meta("linux-1", "ThinkPad", DeviceRole::Desktop),
        )
        .expect("resp2");
        let m2 = resp2.step(&m1).expect("m2").unwrap();
        let m3 = initiator.step(&m2).expect("m3").unwrap();
        resp2.step(&m3).expect("m3");

        let raw_code = initiator.verification_code().expect("code").to_string();
        let dbg_session = format!("{initiator:?}");
        assert!(!dbg_session.contains(&raw_code));
        assert!(dbg_session.contains("[REDACTED]"));

        let export = initiator.approve_and_build_export().expect("first approve");
        assert!(matches!(
            initiator.approve_and_build_export(),
            Err(KeynearError::Validation {
                field: "pairingState",
                ..
            })
        ));

        let resp_export = resp2.approve_and_build_export().expect("resp approve");
        let done = resp2
            .complete_with_peer_export(&export)
            .expect("resp complete");
        let _ = initiator
            .complete_with_peer_export(&resp_export)
            .expect("init complete");
        let secret_hex = hex::encode(done.reconnect_secret);
        let dbg_done = format!("{done:?}");
        assert!(!dbg_done.contains(&secret_hex));
        assert!(dbg_done.contains("[REDACTED]"));

        assert!(resp2.reject().is_err());
    }
}
