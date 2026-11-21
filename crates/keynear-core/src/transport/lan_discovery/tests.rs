use super::*;
use mdns_sd::Error as MdnsError;
use std::net::Ipv4Addr;

#[tokio::test]
async fn handles_isolated_port_and_degraded_monitor() {
    let probe = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("ephemeral udp");
    let port = probe.local_addr().expect("local addr").port();
    drop(probe);

    let discovery = MdnsDiscovery::new_with_port(port);
    assert!(!discovery.is_degraded());

    let registered = discovery
        .register(
            "linux-1",
            &[0x42; 32],
            49152,
            &[IpAddr::V4(Ipv4Addr::LOCALHOST)],
            0x03,
        )
        .expect("register returns Ok even on sandboxed sockets");
    assert!(registered.is_some());

    assert!(
        discovery.handle_monitor_event(&DaemonEvent::Error(MdnsError::Msg(
            "multicast unavailable".to_string(),
        )))
    );
    assert!(discovery.is_degraded());
    assert!(
        discovery.handle_monitor_event(&DaemonEvent::Error(MdnsError::Msg(
            "repeated multicast error".to_string(),
        )))
    );
    assert!(discovery.is_degraded());

    let eid_hex = hex::encode([0x55; 32]);
    let props = [("id", "pixel-9"), ("eid", eid_hex.as_str()), ("cap", "7")];
    let info = ServiceInfo::new(
        KEYNEAR_MDNS_SERVICE_TYPE,
        "keynear-pixel-9",
        "pixel-9.local.",
        &[IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))][..],
        45000,
        &props[..],
    )
    .expect("construct service info");
    let resolved = info.as_resolved_service();
    let peer = parse_resolved_service(&resolved).expect("parse valid resolved service");
    assert_eq!(peer.device_id, "pixel-9");
    assert_eq!(peer.endpoint_id, vec![0x55; 32]);
    assert_eq!(peer.capability_flags, 7);
    assert_eq!(
        peer.socket_addrs,
        vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
            45000
        )]
    );

    discovery.shutdown();
}
