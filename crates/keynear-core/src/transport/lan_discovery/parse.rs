use super::*;

pub fn parse_resolved_service(resolved: &ResolvedService) -> Option<DiscoveredLanPeer> {
    let raw_id = resolved
        .txt_properties
        .get_property_val_str(TXT_DEVICE_ID)?;
    let device_id = sanitize_device_id(raw_id).ok()?;

    let eid_hex = resolved
        .txt_properties
        .get_property_val_str(TXT_ENDPOINT_ID)?;
    let endpoint_id = hex::decode(eid_hex.trim()).ok()?;
    if endpoint_id.len() != 32 {
        return None;
    }

    let capability_flags = resolved
        .txt_properties
        .get_property_val_str(TXT_CAPABILITY_FLAGS)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let mut socket_addrs: Vec<SocketAddr> = resolved
        .addresses
        .iter()
        .map(|scoped| {
            let ip = match scoped {
                ScopedIp::V4(v4) => IpAddr::V4(*v4.addr()),
                ScopedIp::V6(v6) => IpAddr::V6(*v6.addr()),
                other => other.to_ip_addr(),
            };
            SocketAddr::new(ip, resolved.port)
        })
        .collect();
    socket_addrs.sort();
    socket_addrs.dedup();
    if socket_addrs.is_empty() || resolved.port == 0 {
        return None;
    }

    Some(DiscoveredLanPeer {
        device_id,
        endpoint_id,
        socket_addrs,
        capability_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdns_sd::ServiceInfo;
    use std::net::Ipv4Addr;

    #[allow(clippy::too_many_arguments)]
    fn resolved(
        id: Option<&str>,
        eid: &str,
        cap: &str,
        port: u16,
        addresses: Vec<IpAddr>,
    ) -> ResolvedService {
        let mut props: Vec<(&str, &str)> = Vec::new();
        if let Some(id) = id {
            props.push(("id", id));
        }
        props.push(("eid", eid));
        props.push(("cap", cap));
        ServiceInfo::new(
            KEYNEAR_MDNS_SERVICE_TYPE,
            "keynear-test",
            "linux-1.local.",
            &addresses[..],
            port,
            &props[..],
        )
        .expect("construct service info")
        .as_resolved_service()
    }

    fn addresses() -> Vec<IpAddr> {
        vec![
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
        ]
    }

    #[test]
    fn parse_resolved_service_sanitizes_id_and_sorts_dedups_addresses() {
        let service = resolved(
            Some("  pixel-9_pro.1  "),
            &"55".repeat(32),
            "7",
            45000,
            addresses(),
        );
        let peer = parse_resolved_service(&service).expect("valid service");
        assert_eq!(peer.device_id, "pixel-9_pro.1");
        assert_eq!(peer.endpoint_id, vec![0x55; 32]);
        assert_eq!(peer.capability_flags, 7);
        assert_eq!(
            peer.socket_addrs,
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 45000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)), 45000),
            ]
        );
    }

    #[test]
    fn parse_resolved_service_rejects_missing_or_invalid_device_id() {
        let eid = "55".repeat(32);
        assert!(parse_resolved_service(&resolved(None, &eid, "1", 45000, addresses())).is_none());
        assert!(parse_resolved_service(&resolved(
            Some("../etc/passwd"),
            &eid,
            "1",
            45000,
            addresses()
        ))
        .is_none());
        assert!(
            parse_resolved_service(&resolved(Some("bad/id"), &eid, "1", 45000, addresses()))
                .is_none()
        );
    }

    #[test]
    fn parse_resolved_service_rejects_invalid_endpoint_ids() {
        assert!(parse_resolved_service(&resolved(
            Some("pixel-9"),
            "zzzz",
            "1",
            45000,
            addresses()
        ))
        .is_none());
        assert!(parse_resolved_service(&resolved(
            Some("pixel-9"),
            "aabb",
            "1",
            45000,
            addresses()
        ))
        .is_none());
        assert!(parse_resolved_service(&resolved(
            Some("pixel-9"),
            &"55".repeat(64),
            "1",
            45000,
            addresses()
        ))
        .is_none());
    }

    #[test]
    fn parse_resolved_service_rejects_zero_port_and_empty_addresses() {
        let eid = "55".repeat(32);
        assert!(
            parse_resolved_service(&resolved(Some("pixel-9"), &eid, "1", 0, addresses())).is_none()
        );
        assert!(
            parse_resolved_service(&resolved(Some("pixel-9"), &eid, "1", 45000, Vec::new()))
                .is_none()
        );
    }

    #[test]
    fn parse_resolved_service_defaults_unparsable_capability_flags() {
        let service = resolved(
            Some("pixel-9"),
            &"55".repeat(32),
            "many",
            45000,
            addresses(),
        );
        let peer = parse_resolved_service(&service).expect("valid service");
        assert_eq!(peer.capability_flags, 0);
    }
}
