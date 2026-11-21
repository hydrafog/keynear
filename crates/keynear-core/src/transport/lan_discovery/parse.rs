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
