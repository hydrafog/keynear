use super::*;
use std::net::{IpAddr, Ipv4Addr};

fn loopback_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}

fn node_config(device_id: &str, seed_byte: u8) -> OrionNodeConfig {
    OrionNodeConfig {
        device_id: device_id.to_string(),
        display_name: format!("Node {device_id}"),
        capability_flags: 0x03,
        quic_secret_key: Zeroizing::new([seed_byte; 32]),
    }
}

#[tokio::test]
async fn endpoint_disables_relays_and_negotiates_x25519mlkem768() {
    let provider = build_orion_crypto_provider();
    assert_eq!(provider.kx_groups.len(), 1);
    assert_eq!(
        provider.kx_groups[0].name(),
        aws_lc_rs::kx_group::X25519MLKEM768.name()
    );

    let server = OrionQuicNode::bind(node_config("linux-host", 0x11), Some(loopback_addr()))
        .await
        .expect("bind server");
    let client = OrionQuicNode::bind(node_config("pixel-phone", 0x22), Some(loopback_addr()))
        .await
        .expect("bind client");

    assert!(server.endpoint_addr().relay_urls().next().is_none());
    assert!(client.endpoint_addr().relay_urls().next().is_none());

    let secret = [0x99; 32];
    let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();
    let client_addrs: Vec<SocketAddr> = client.endpoint_addr().ip_addrs().copied().collect();
    assert!(!server_addrs.is_empty());

    server
        .register_peer("pixel-phone", client.endpoint_id(), client_addrs, secret)
        .expect("register client");
    client
        .register_peer(
            "linux-host",
            server.endpoint_id(),
            server_addrs.clone(),
            secret,
        )
        .expect("register server");

    let server_clone = server.clone();
    let accept_task = tokio::spawn(async move { server_clone.accept_authenticated().await });

    let conn = client
        .connect_peer("linux-host")
        .await
        .expect("client connects over PQ QUIC");
    assert_eq!(conn.alpn(), QUIC_ALPN);

    let accepted = accept_task
        .await
        .expect("server join")
        .expect("server accept")
        .expect("some conn");
    assert_eq!(accepted.0, "pixel-phone");

    let mut classic_provider = aws_lc_rs::default_provider();
    classic_provider.kx_groups = vec![aws_lc_rs::kx_group::X25519];
    let classic_client = Endpoint::builder(presets::Empty)
        .secret_key(SecretKey::from_bytes(&[0x33; 32]))
        .crypto_provider(Arc::new(classic_provider))
        .relay_mode(RelayMode::Disabled)
        .bind_addr(loopback_addr())
        .expect("bind addr")
        .bind()
        .await
        .expect("bind classic client");
    server
        .register_peer("classic-peer", classic_client.id(), vec![], [0x44; 32])
        .expect("register classic peer");
    let server_addr = EndpointAddr::from_parts(
        server.endpoint_id(),
        server_addrs.into_iter().map(TransportAddr::Ip),
    );
    assert!(
        classic_client
            .connect(server_addr, QUIC_ALPN)
            .await
            .is_err(),
        "classic-only peer without X25519MLKEM768 must be rejected"
    );

    classic_client.close().await;
    client.close().await;
    server.close().await;
}

#[tokio::test]
async fn rejects_unpaired_and_invalid_hmac() {
    let server = OrionQuicNode::bind(node_config("linux-host", 0x51), Some(loopback_addr()))
        .await
        .expect("bind server");
    let rogue = OrionQuicNode::bind(node_config("rogue-node", 0x52), Some(loopback_addr()))
        .await
        .expect("bind rogue");
    let paired = OrionQuicNode::bind(node_config("pixel-phone", 0x53), Some(loopback_addr()))
        .await
        .expect("bind paired");

    let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();

    rogue
        .register_peer(
            "linux-host",
            server.endpoint_id(),
            server_addrs.clone(),
            [0xAA; 32],
        )
        .expect("rogue registers server");
    let server_for_rogue = server.clone();
    let rogue_accept = tokio::spawn(async move {
        if let Some(incoming) = server_for_rogue.endpoint().accept().await {
            if let Ok(connecting) = incoming.accept() {
                return connecting.await.is_ok();
            }
        }
        false
    });
    let rogue_res = rogue.connect_peer("linux-host").await;
    assert!(rogue_res.is_err(), "unpaired peer must be rejected");
    let _ = rogue_accept.await;

    server
        .register_peer("pixel-phone", paired.endpoint_id(), vec![], [0x11; 32])
        .expect("server registers paired");
    paired
        .register_peer("linux-host", server.endpoint_id(), server_addrs, [0x22; 32])
        .expect("paired registers server with wrong secret");

    let server_for_hmac = server.clone();
    let hmac_accept = tokio::spawn(async move { server_for_hmac.accept_authenticated().await });
    let client_res = paired.connect_peer("linux-host").await;
    assert!(
        client_res.is_err(),
        "mismatched reconnect_secret must fail handshake auth, got {client_res:?}"
    );
    let server_res = hmac_accept.await.expect("join");
    assert!(
        matches!(server_res, Err(KeynearError::UnauthorizedPeer(_))),
        "server must reject invalid HMAC auth_proof"
    );

    rogue.close().await;
    paired.close().await;
    server.close().await;
}

#[tokio::test]
async fn send_proximity_state_roundtrips_and_peer_lifecycle_works() {
    let server = OrionQuicNode::bind(node_config("linux-host", 0x61), Some(loopback_addr()))
        .await
        .expect("bind server");
    let client = OrionQuicNode::bind(node_config("pixel-phone", 0x62), Some(loopback_addr()))
        .await
        .expect("bind client");
    let secret = [0x77; 32];
    let server_ep = server.local_lan_endpoint();
    assert_eq!(server_ep.device_id, "linux-host");
    let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();
    let client_addrs: Vec<SocketAddr> = client.endpoint_addr().ip_addrs().copied().collect();

    server
        .register_peer("pixel-phone", client.endpoint_id(), client_addrs, secret)
        .expect("register client");
    client
        .register_peer("linux-host", server.endpoint_id(), server_addrs, secret)
        .expect("register server");
    assert_eq!(
        server.peer_device_id(&client.endpoint_id()).as_deref(),
        Some("pixel-phone")
    );

    let server_clone = server.clone();
    let recv_task = tokio::spawn(async move {
        let (_peer, conn) = server_clone
            .accept_authenticated()
            .await
            .expect("accept")
            .expect("conn");
        let (mut send, mut recv) = conn.accept_bi().await.expect("bi");
        let kind = read_stream_kind(&mut recv).await.expect("kind");
        assert_eq!(kind, QuicStreamKind::Control);
        let env = read_control_envelope(&mut recv).await.expect("env");
        let ack = v1::Envelope {
            version: 1,
            sequence: env.sequence,
            timestamp_ms: 1,
            payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
                acked_sequence: env.sequence,
                accepted: true,
                reason: String::new(),
            })),
        };
        write_control_envelope(&mut send, &ack).await.expect("ack");
        let _ = send.finish();
        env.payload
    });

    let state = v1::ProximityState {
        device_id: "pixel-phone".to_string(),
        status: v1::ProximityStatus::ProximityInRange as i32,
        distance_meters: 4.5,
        session_locked: false,
        sequence: 1,
    };
    client
        .send_proximity_state("linux-host", state.clone())
        .await
        .expect("send proximity state");
    let got = recv_task.await.expect("join");
    assert_eq!(got, Some(v1::envelope::Payload::ProximityState(state)));

    client.invalidate_peer_connection("linux-host").await;
    let secrets = crate::InMemorySecretStore::new();
    let device_list = vec![
        PairedDevice {
            device_id: "skip-empty".to_string(),
            display_name: "Skip".to_string(),
            device_role: keynear_protocol::DeviceRole::Mobile,
            static_x25519_pubkey: "aa".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        },
        PairedDevice {
            device_id: "skip-bad-hex".to_string(),
            display_name: "Bad".to_string(),
            device_role: keynear_protocol::DeviceRole::Mobile,
            static_x25519_pubkey: "aa".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            last_endpoint_id: "zzzz".to_string(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        },
    ];
    server
        .sync_paired_devices(&device_list, &secrets)
        .expect("sync paired devices");
    server.remove_peer("pixel-phone").expect("remove peer");
    assert!(server.peer_device_id(&client.endpoint_id()).is_none());

    client.close().await;
    server.close().await;
}
