use super::*;

fn item(name: &str, size_bytes: u64) -> TransferItem {
    TransferItem {
        name: name.to_string(),
        mime_type: "text/plain".to_string(),
        size_bytes,
        is_directory: false,
        target_path: None,
        checksum: None,
    }
}

fn offer(transfer_id: &str, name: &str, size_bytes: u64) -> v1::TransferOffer {
    v1::TransferOffer {
        transfer_id: transfer_id.to_string(),
        items: vec![v1::TransferItemMeta {
            name: name.to_string(),
            mime_type: "text/plain".to_string(),
            size_bytes,
            is_directory: false,
        }],
        total_bytes: size_bytes,
    }
}

#[test]
fn new_registry_starts_empty() {
    let registry = TransferRegistry::new();
    assert!(registry.list_sessions().is_empty());
    assert!(registry.get_session("missing").is_none());
}

#[test]
fn register_outbound_uses_a_fresh_cancel_flag_and_active_state() {
    let registry = TransferRegistry::new();
    let cancel = registry
        .register_outbound(
            "tx-1".to_string(),
            "peer-1".to_string(),
            vec![item("a.txt", 7)],
            7,
        )
        .expect("register outbound");
    assert!(!cancel.load(Ordering::Acquire));
    let session = registry.get_session("tx-1").expect("session");
    assert_eq!(session.direction, TransferDirection::Send);
    assert_eq!(session.state, TransferState::Active);
    assert_eq!(session.peer_device_id, "peer-1");
    assert_eq!(session.total_bytes, 7);
    assert_eq!(session.items.len(), 1);
}

#[test]
fn list_sessions_orders_by_start_time_then_id() {
    let registry = TransferRegistry::new();
    for id in ["tx-b", "tx-a", "tx-c"] {
        registry
            .register_outbound(id.to_string(), "peer-1".to_string(), Vec::new(), 0)
            .expect("register outbound");
    }
    let list = registry.list_sessions();
    assert_eq!(list.len(), 3);
    for pair in list.windows(2) {
        let earlier = &pair[0];
        let later = &pair[1];
        assert!(
            earlier.started_at > later.started_at
                || (earlier.started_at == later.started_at
                    && earlier.transfer_id <= later.transfer_id)
        );
    }
}

#[test]
fn register_outbound_rejects_an_invalid_peer_id() {
    let registry = TransferRegistry::new();
    let err = registry
        .register_outbound("tx-1".to_string(), "../bad".to_string(), Vec::new(), 0)
        .expect_err("invalid peer id");
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "deviceId"));
    assert!(registry.get_session("tx-1").is_none());
}

#[test]
fn register_inbound_sets_state_and_channel_availability_from_policy() {
    let registry = TransferRegistry::new();
    let auto = registry
        .register_inbound(
            "peer-1".to_string(),
            &offer("tx-auto", "a.txt", 7),
            true,
            false,
        )
        .expect("register auto accept");
    assert_eq!(auto.state, TransferState::Active);
    assert!(auto.consent_rx.is_none());
    assert!(auto.fd_rx.is_none());

    let pending = registry
        .register_inbound(
            "peer-2".to_string(),
            &offer("tx-pending", "b.txt", 9),
            false,
            true,
        )
        .expect("register pending");
    assert_eq!(pending.state, TransferState::PendingConsent);
    assert!(pending.consent_rx.is_some());
    assert!(pending.fd_rx.is_some());
    assert_eq!(
        registry.get_session("tx-pending").expect("session").state,
        TransferState::PendingConsent
    );
}

#[test]
fn register_inbound_rejects_duplicate_ids_and_invalid_offers() {
    let registry = TransferRegistry::new();
    registry
        .register_inbound(
            "peer-1".to_string(),
            &offer("tx-1", "a.txt", 7),
            true,
            false,
        )
        .expect("first registration");
    let err = match registry.register_inbound(
        "peer-2".to_string(),
        &offer("tx-1", "b.txt", 9),
        true,
        false,
    ) {
        Ok(_) => panic!("expected duplicate id rejection"),
        Err(err) => err,
    };
    assert!(
        matches!(err, KeynearError::Transfer(message) if message.contains("duplicate transfer_id"))
    );
    let err = match registry.register_inbound(
        "peer-1".to_string(),
        &offer("", "b.txt", 9),
        true,
        false,
    ) {
        Ok(_) => panic!("expected invalid offer rejection"),
        Err(err) => err,
    };
    assert!(matches!(err, KeynearError::Validation { field, .. } if field == "transferId"));
}

#[tokio::test]
async fn respond_consent_accept_moves_to_active_and_signals_true() {
    let registry = TransferRegistry::new();
    let registration = registry
        .register_inbound(
            "peer-1".to_string(),
            &offer("tx-1", "a.txt", 7),
            false,
            false,
        )
        .expect("register");
    let session = registry.respond_consent("tx-1", true).expect("accept");
    assert_eq!(session.state, TransferState::Active);
    assert!(registration
        .consent_rx
        .expect("consent rx")
        .await
        .expect("consent signal"));
    assert!(matches!(
        registry.respond_consent("tx-1", true),
        Err(KeynearError::Transfer(message)) if message.contains("not awaiting consent")
    ));
}

#[tokio::test]
async fn respond_consent_reject_cancels_and_signals_false() {
    let registry = TransferRegistry::new();
    let registration = registry
        .register_inbound(
            "peer-1".to_string(),
            &offer("tx-1", "a.txt", 7),
            false,
            true,
        )
        .expect("register");
    let session = registry.respond_consent("tx-1", false).expect("reject");
    assert_eq!(session.state, TransferState::Cancelled);
    assert!(registration.cancel_flag.load(Ordering::Acquire));
    assert!(!registration
        .consent_rx
        .expect("consent rx")
        .await
        .expect("consent signal"));
    assert!(registration.fd_rx.expect("fd rx").try_recv().is_err());
    assert!(matches!(
        registry.respond_consent("unknown", true),
        Err(KeynearError::Transfer(message)) if message.contains("not found")
    ));
}

#[tokio::test]
async fn provide_receive_fd_delivers_once_and_rejects_terminated_transfers() {
    let registry = TransferRegistry::new();
    let registration = registry
        .register_inbound("peer-1".to_string(), &offer("tx-1", "a.txt", 7), true, true)
        .expect("register");
    let path = std::env::temp_dir().join("keynear-registry-fd-test.txt");
    let file = File::create(&path).expect("create temp file");
    registry
        .provide_receive_fd("tx-1", file)
        .expect("provide fd");
    let received = registration.fd_rx.expect("fd rx").await.expect("fd");
    assert!(received.metadata().expect("metadata").is_file());
    drop(received);
    let _ = std::fs::remove_file(&path);
    assert!(matches!(
        registry.provide_receive_fd("tx-1", File::create(&path).expect("second temp")),
        Err(KeynearError::Transfer(message)) if message.contains("not awaiting")
    ));
    let _ = std::fs::remove_file(&path);

    let terminated = registry
        .register_inbound("peer-1".to_string(), &offer("tx-2", "b.txt", 7), true, true)
        .expect("register terminated");
    registry.finish_transfer("tx-2", TransferState::Completed, 7, None, None);
    let err = registry
        .provide_receive_fd("tx-2", File::create(&path).expect("third temp"))
        .expect_err("terminated transfer");
    assert!(matches!(err, KeynearError::TransferCancelled(_)));
    assert!(terminated.fd_rx.is_some());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cancel_transfer_preserves_terminal_states_and_signals_waiters() {
    let registry = TransferRegistry::new();
    let registration = registry
        .register_inbound(
            "peer-1".to_string(),
            &offer("tx-1", "a.txt", 7),
            false,
            true,
        )
        .expect("register");
    let session = registry.cancel_transfer("tx-1").expect("cancel");
    assert_eq!(session.state, TransferState::Cancelled);
    assert!(registration.cancel_flag.load(Ordering::Acquire));
    assert!(registration.fd_rx.expect("fd rx").try_recv().is_err());

    registry
        .register_outbound("tx-2".to_string(), "peer-1".to_string(), Vec::new(), 0)
        .expect("register outbound");
    registry.finish_transfer("tx-2", TransferState::Completed, 0, None, None);
    let completed = registry.cancel_transfer("tx-2").expect("cancel completed");
    assert_eq!(completed.state, TransferState::Completed);
}

#[test]
fn progress_and_finish_record_bytes_paths_and_checksum() {
    let registry = TransferRegistry::new();
    registry
        .register_outbound(
            "tx-1".to_string(),
            "peer-1".to_string(),
            vec![item("a.txt", 4), item("b.txt", 6)],
            10,
        )
        .expect("register outbound");
    registry.update_progress("tx-1", 5);
    assert_eq!(
        registry
            .get_session("tx-1")
            .expect("session")
            .transferred_bytes,
        5
    );
    registry.finish_transfer(
        "tx-1",
        TransferState::Completed,
        10,
        Some(&["/dest/a.txt".to_string(), "/dest/b.txt".to_string()]),
        Some("deadbeef"),
    );
    let session = registry.get_session("tx-1").expect("session");
    assert_eq!(session.state, TransferState::Completed);
    assert_eq!(session.transferred_bytes, 10);
    assert_eq!(session.items[0].target_path.as_deref(), Some("/dest/a.txt"));
    assert_eq!(session.items[1].target_path.as_deref(), Some("/dest/b.txt"));
    assert!(session
        .items
        .iter()
        .all(|entry| entry.checksum.as_deref() == Some("deadbeef")));
    registry.update_progress("unknown", 1);
}
