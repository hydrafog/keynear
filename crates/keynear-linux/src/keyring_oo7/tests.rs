use std::fs;
use std::path::PathBuf;

use keynear_core::keyring::PeerSecrets;

use super::*;
use keynear_core::keyring::SecretStore;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_config_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-oo7-{label}-{nanos}"))
}

#[test]
fn file_keyring_persists_keys_and_peer_secrets() {
    let dir = temp_config_dir("fallback");
    let store = Oo7SecretStore::open_file_only(&dir).expect("open file keyring");
    assert!(store.is_file_fallback());

    let kp1 = store
        .get_or_create_static_keypair()
        .expect("create keypair");
    let kp2 = store
        .get_or_create_static_keypair()
        .expect("reload keypair");
    assert_eq!(kp1.public_key, kp2.public_key);
    assert_eq!(kp1.private_key, kp2.private_key);

    let qk1 = store
        .get_or_create_quic_secret_key()
        .expect("create quic key");
    let qk2 = store
        .get_or_create_quic_secret_key()
        .expect("reload quic key");
    assert_eq!(*qk1, *qk2);
    assert_ne!(*qk1, [0u8; 32]);

    let secrets = PeerSecrets {
        irk: [0x33; 32],
        reconnect_secret: [0x44; 32],
    };
    store
        .put_peer_secrets("pixel-9", &secrets)
        .expect("put peer secrets");
    assert_eq!(
        store.get_peer_secrets("pixel-9").expect("get peer secrets"),
        Some(secrets.clone())
    );
    assert_eq!(store.list_peer_ids().expect("list peers"), vec!["pixel-9"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_keyring_persists_counters_and_reopens() {
    let dir = temp_config_dir("counter");
    let store = Oo7SecretStore::open_file_only(&dir).expect("open file keyring");
    let c1 = store.next_presence_counter().expect("c1");
    let c2 = store.next_presence_counter().expect("c2");
    assert_eq!(c1, 1);
    assert_eq!(c2, 2);
    let qk1 = store.get_or_create_quic_secret_key().expect("quic key");
    let secrets = PeerSecrets {
        irk: [0x33; 32],
        reconnect_secret: [0x44; 32],
    };
    store
        .put_peer_secrets("pixel-9", &secrets)
        .expect("put peer secrets");
    drop(store);

    let reopened = Oo7SecretStore::open_file_only(&dir).expect("reopen file keyring");
    assert_eq!(
        *reopened
            .get_or_create_quic_secret_key()
            .expect("persisted quic key"),
        *qk1
    );
    assert_eq!(
        reopened.get_peer_secrets("pixel-9").expect("persisted"),
        Some(secrets)
    );
    assert_eq!(reopened.next_presence_counter().expect("c3"), 3);
    reopened
        .delete_peer_secrets("pixel-9")
        .expect("delete peer secrets");
    assert!(reopened.get_peer_secrets("pixel-9").unwrap().is_none());
    let _ = fs::remove_dir_all(&dir);
}
