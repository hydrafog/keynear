use super::*;

#[cfg(unix)]
#[test]
fn owned_fd_closes_once_on_drop() {
    use std::io::{ErrorKind, Read};
    use std::os::fd::IntoRawFd;
    use std::os::unix::net::UnixStream;

    assert!(matches!(
        wrap_raw_fd(-1),
        Err(KeynearError::Validation { field: "rawFd", .. })
    ));

    let (mut peer_a, peer_b) = UnixStream::pair().expect("create unix stream pair");
    peer_a.set_nonblocking(true).expect("set nonblocking");
    let raw_fd = peer_b.into_raw_fd();

    {
        let _owned = wrap_raw_fd(raw_fd).expect("wrap valid raw fd");
        let mut buf = [0u8; 1];
        let err = peer_a
            .read(&mut buf)
            .expect_err("peer fd must remain open while owned guard is alive");
        assert_eq!(err.kind(), ErrorKind::WouldBlock);
    }

    peer_a
        .set_nonblocking(false)
        .expect("restore blocking mode");
    peer_a
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set read timeout");
    let mut buf = [0u8; 1];
    let n = peer_a
        .read(&mut buf)
        .expect("peer read must observe EOF once owned fd drops");
    assert_eq!(n, 0);
}

#[cfg(unix)]
#[test]
fn orion_node_handle_transfers_via_fd_and_cleans_up() {
    use std::io::Write;
    use std::net::{IpAddr, Ipv4Addr};
    use std::os::fd::IntoRawFd;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-orion-ffi-{nonce}"));
    let recv_dir = dir.join("recv");
    fs::create_dir_all(&recv_dir).expect("mkdir");

    let loopback = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let receiver = OrionNodeHandle::build_with_init(OrionNodeInit {
        device_id: "rx-node".to_string(),
        display_name: "Receiver".to_string(),
        capability_flags: 1,
        quic_secret_key: vec![0x71; 32],
        incoming_directory: recv_dir.to_string_lossy().to_string(),
        auto_accept_from_paired: false,
        require_explicit_fd_for_single_file: true,
        bind_addr: Some(loopback),
    })
    .expect("bind receiver");

    let sender = OrionNodeHandle::build_with_init(OrionNodeInit {
        device_id: "tx-node".to_string(),
        display_name: "Sender".to_string(),
        capability_flags: 1,
        quic_secret_key: vec![0x72; 32],
        incoming_directory: dir.to_string_lossy().to_string(),
        auto_accept_from_paired: false,
        require_explicit_fd_for_single_file: false,
        bind_addr: Some(loopback),
    })
    .expect("bind sender");

    let secret = vec![0x9A; 32];
    let rx_ep = receiver.local_lan_endpoint();
    let tx_ep = sender.local_lan_endpoint();

    receiver
        .sync_peer(
            "tx-node".to_string(),
            tx_ep.endpoint_id,
            tx_ep.socket_addrs,
            secret.clone(),
            true,
        )
        .expect("rx sync");
    sender
        .sync_peer(
            "rx-node".to_string(),
            rx_ep.endpoint_id,
            rx_ep.socket_addrs,
            secret,
            false,
        )
        .expect("tx sync");

    let src_path = dir.join("send.txt");
    let out_path = recv_dir.join("received.txt");
    let payload = b"zero-copy OwnedFd UniFFI stream";
    {
        let mut f = fs::File::create(&src_path).expect("create src");
        f.write_all(payload).expect("write src");
    }
    let send_file = fs::File::open(&src_path).expect("open src");
    let send_raw_fd = send_file.into_raw_fd();

    let tx_thread = thread::spawn(move || {
        let res = sender.send_fd(
            "rx-node".to_string(),
            "send.txt".to_string(),
            "text/plain".to_string(),
            payload.len() as u64,
            send_raw_fd,
        );
        sender.shutdown();
        res
    });

    let transfer_id = loop {
        if let Some(session) = receiver
            .list_transfers()
            .into_iter()
            .find(|s| s.state == FfiTransferState::Active)
        {
            break session.transfer_id;
        }
        thread::sleep(Duration::from_millis(10));
    };

    let out_file = fs::File::create(&out_path).expect("create out");
    receiver
        .provide_receive_fd(transfer_id.clone(), out_file.into_raw_fd())
        .expect("provide fd");

    let sent = tx_thread.join().expect("join tx").expect("send_fd ok");
    assert_eq!(sent.state, FfiTransferState::Completed);

    loop {
        if let Some(session) = receiver
            .list_transfers()
            .into_iter()
            .find(|s| s.transfer_id == transfer_id)
        {
            if session.state == FfiTransferState::Completed {
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(fs::read(&out_path).expect("read received"), payload);
    receiver.shutdown();
    let _ = fs::remove_dir_all(&dir);
}
