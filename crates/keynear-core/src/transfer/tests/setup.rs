use super::*;

pub(super) fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-transfer-{label}-{nanos}"))
}

pub(super) fn loopback_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}

pub(super) async fn connected_pair() -> (OrionQuicNode, OrionQuicNode, Connection, Connection) {
    let server = OrionQuicNode::bind(
        OrionNodeConfig {
            device_id: "server-1".to_string(),
            display_name: "Server".to_string(),
            capability_flags: 1,
            quic_secret_key: Zeroizing::new([0x61; 32]),
        },
        Some(loopback_addr()),
    )
    .await
    .expect("bind server");
    let client = OrionQuicNode::bind(
        OrionNodeConfig {
            device_id: "client-1".to_string(),
            display_name: "Client".to_string(),
            capability_flags: 1,
            quic_secret_key: Zeroizing::new([0x62; 32]),
        },
        Some(loopback_addr()),
    )
    .await
    .expect("bind client");

    let secret = [0xAB; 32];
    let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();
    let client_addrs: Vec<SocketAddr> = client.endpoint_addr().ip_addrs().copied().collect();

    server
        .register_peer("client-1", client.endpoint_id(), client_addrs, secret)
        .expect("register client");
    client
        .register_peer("server-1", server.endpoint_id(), server_addrs, secret)
        .expect("register server");

    let server_clone = server.clone();
    let accept_task = tokio::spawn(async move { server_clone.accept_authenticated().await });
    let client_conn = client.connect_peer("server-1").await.expect("connect");
    let (_, server_conn) = accept_task
        .await
        .expect("join")
        .expect("accept")
        .expect("conn");

    (server, client, server_conn, client_conn)
}
