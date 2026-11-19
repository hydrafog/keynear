pub mod history;
pub mod otp;
pub mod sync;

pub use history::{
    is_image_mime, normalize_mime_type, pick_preferred_image_mime, should_ignore_mime_offers,
    ClipboardItem, ClipboardPolicy, DelayedClipboardOffer, DelayedRenderCallback,
    BUILT_IN_IGNORED_MIMES, CLIPBOARD_OFFER_TTL_SECS, DEFAULT_TEXT_MIME, MAX_CLIPBOARD_IMAGE_BYTES,
    MAX_CLIPBOARD_QUEUE, MAX_INLINE_CLIPBOARD_BYTES,
};
pub use sync::{read_clipboard_blob, write_clipboard_blob, ClipboardEngine};

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use keynear_protocol::{read_stream_kind, KeynearError};
    use zeroize::Zeroizing;

    use crate::transport::{OrionNodeConfig, OrionQuicNode};

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

    async fn connected_pair() -> (OrionQuicNode, OrionQuicNode) {
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

    fn spawn_clipboard_accept_loop(
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
                                        .handle_incoming_stream_with_kind(
                                            &peer_id, kind, send, recv,
                                        )
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

    #[tokio::test]
    async fn delayed_image_offer_defers_payload_and_bounds_blob_stream() {
        let (sender_node, receiver_node) = connected_pair().await;
        let sender_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));
        let receiver_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));

        let sender_task =
            spawn_clipboard_accept_loop(sender_node.clone(), Arc::clone(&sender_engine));
        let receiver_task =
            spawn_clipboard_accept_loop(receiver_node.clone(), Arc::clone(&receiver_engine));

        let render_calls = Arc::new(AtomicUsize::new(0));
        let render_calls_cb = Arc::clone(&render_calls);
        let large_png = vec![0x89u8; 96 * 1024];
        let large_png_cb = large_png.clone();

        let offer = sender_engine
            .offer_clipboard_image(
                &sender_node,
                "pixel-phone",
                vec!["image/png".to_string(), "image/jpeg".to_string()],
                Arc::new(move |mime| {
                    assert_eq!(mime, "image/png");
                    render_calls_cb.fetch_add(1, Ordering::SeqCst);
                    Ok(large_png_cb.clone())
                }),
            )
            .await
            .expect("offer image");

        assert_eq!(
            render_calls.load(Ordering::SeqCst),
            0,
            "image payload must not render when publishing ClipboardOffer"
        );

        let queued = receiver_engine.poll_incoming();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].is_image_offer);
        assert!(queued[0].data.is_empty());
        assert_eq!(queued[0].offer_id, offer.offer_id);

        let fetched = receiver_engine
            .fetch_clipboard_image(&receiver_node, "linux-host", &offer.offer_id, "image/png")
            .await
            .expect("fetch delayed image");

        assert_eq!(render_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fetched.mime_type, "image/png");
        assert_eq!(fetched.data, large_png);

        sender_task.abort();
        receiver_task.abort();
        sender_node.close().await;
        receiver_node.close().await;
    }

    #[tokio::test]
    async fn password_mime_and_disabled_otp_are_rejected_without_queueing() {
        let engine = ClipboardEngine::new(ClipboardPolicy::default());
        let render_calls = Arc::new(AtomicUsize::new(0));
        let render_calls_cb = Arc::clone(&render_calls);

        let err = engine
            .register_delayed_offer(
                vec![
                    "text/plain".to_string(),
                    "x-kde-passwordManagerHint".to_string(),
                ],
                false,
                false,
                0,
                Arc::new(move |_| {
                    render_calls_cb.fetch_add(1, Ordering::SeqCst);
                    Ok(b"secret".to_vec())
                }),
            )
            .expect_err("password MIME offer must be rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
        assert_eq!(render_calls.load(Ordering::SeqCst), 0);

        engine.set_policy(ClipboardPolicy {
            otp_passthrough_enabled: false,
            ..Default::default()
        });
        assert!(!engine.can_accept_mimes(&["text/plain".to_string()], false, true));
    }
}
