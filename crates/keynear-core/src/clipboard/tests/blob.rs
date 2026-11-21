use super::*;

#[test]
fn should_ignore_mime_offers_handles_builtin_user_case_and_params() {
    let user = vec!["Application/X-Secret".to_string()];

    assert!(should_ignore_mime_offers(
        &["x-kde-passwordManagerHint".to_string()],
        &[]
    ));
    assert!(should_ignore_mime_offers(
        &["X-KDE-PASSWORDMANAGERHINT; charset=utf-8".to_string()],
        &[]
    ));
    assert!(should_ignore_mime_offers(
        &[
            "text/plain".to_string(),
            "application/x-password".to_string()
        ],
        &[]
    ));
    assert!(should_ignore_mime_offers(
        &["  application/x-secret ; mode=ro ".to_string()],
        &user
    ));
    assert!(!should_ignore_mime_offers(
        &[
            "text/plain;charset=utf-8".to_string(),
            "image/png".to_string()
        ],
        &user
    ));
    assert!(!should_ignore_mime_offers(&[], &["".to_string()]));
}

#[tokio::test]
async fn clipboard_blob_roundtrip_and_bounds_enforced() {
    let payload = vec![0xAB; 128 * 1024];
    let mut wire = Vec::new();
    write_clipboard_blob(&mut wire, "image/png", &payload, false)
        .await
        .expect("write blob");

    let mut reader = wire.as_slice();
    let (mime, decoded, is_otp) = read_clipboard_blob(&mut reader).await.expect("read blob");
    assert_eq!(mime, "image/png");
    assert_eq!(decoded, payload);
    assert!(!is_otp);

    let oversized = vec![0u8; MAX_CLIPBOARD_IMAGE_BYTES + 1];
    let mut sink = Vec::new();
    let err = write_clipboard_blob(&mut sink, "image/png", &oversized, false)
        .await
        .expect_err("oversized blob must fail");
    assert!(matches!(err, KeynearError::FrameTooLarge { .. }));

    let mut forged_header = Vec::new();
    forged_header.extend_from_slice(&(9u16).to_be_bytes());
    forged_header.extend_from_slice(b"image/png");
    forged_header.push(0);
    forged_header.extend_from_slice(&((MAX_CLIPBOARD_IMAGE_BYTES as u64) + 1).to_be_bytes());
    forged_header.extend_from_slice(&[0u8; 32]);
    let err = read_clipboard_blob(&mut forged_header.as_slice())
        .await
        .expect_err("oversized header length must fail before allocation");
    assert!(matches!(err, KeynearError::FrameTooLarge { .. }));
}

pub(super) async fn connected_pair() -> (OrionQuicNode, OrionQuicNode) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let a = OrionQuicNode::bind(
        OrionNodeConfig {
            device_id: "linux-host".to_string(),
            display_name: "Linux".to_string(),
            capability_flags: 1,
            quic_secret_key: Zeroizing::new([0x11; 32]),
        },
        Some(addr),
    )
    .await
    .expect("bind a");
    let b = OrionQuicNode::bind(
        OrionNodeConfig {
            device_id: "pixel-phone".to_string(),
            display_name: "Pixel".to_string(),
            capability_flags: 1,
            quic_secret_key: Zeroizing::new([0x22; 32]),
        },
        Some(addr),
    )
    .await
    .expect("bind b");

    let secret = [0x5A; 32];
    let a_addrs: Vec<SocketAddr> = a.endpoint_addr().ip_addrs().copied().collect();
    let b_addrs: Vec<SocketAddr> = b.endpoint_addr().ip_addrs().copied().collect();
    a.register_peer("pixel-phone", b.endpoint_id(), b_addrs, secret)
        .expect("register b");
    b.register_peer("linux-host", a.endpoint_id(), a_addrs, secret)
        .expect("register a");
    (a, b)
}

pub(super) fn spawn_clipboard_accept_loop(
    node: OrionQuicNode,
    engine: Arc<ClipboardEngine>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match node.accept_authenticated().await {
                Ok(Some((peer_id, conn))) => {
                    let eng = Arc::clone(&engine);
                    tokio::spawn(async move {
                        while let Ok((send, mut recv)) = conn.accept_bi().await {
                            if let Ok(kind) = read_stream_kind(&mut recv).await {
                                let _ = eng
                                    .handle_incoming_stream_with_kind(&peer_id, kind, send, recv)
                                    .await;
                            }
                        }
                    });
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }
    })
}
