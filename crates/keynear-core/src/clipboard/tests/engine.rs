use super::*;

use super::blob::{connected_pair, spawn_clipboard_accept_loop};

#[tokio::test]
async fn delayed_image_offer_defers_payload_and_bounds_blob_stream() {
    let (sender_node, receiver_node) = connected_pair().await;
    let sender_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));
    let receiver_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));

    let sender_task = spawn_clipboard_accept_loop(sender_node.clone(), Arc::clone(&sender_engine));
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

#[tokio::test]
async fn send_clipboard_text_and_blob_route_to_incoming_sender() {
    let (sender_node, receiver_node) = connected_pair().await;
    let sender_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let receiver_engine = Arc::new(ClipboardEngine::with_incoming_sender(
        ClipboardPolicy::default(),
        tx,
    ));
    let receiver_task = spawn_clipboard_accept_loop(receiver_node.clone(), receiver_engine);

    sender_engine
        .send_clipboard_text(&sender_node, "pixel-phone", "482910", true)
        .await
        .expect("send inline OTP text");
    let inline_item = rx.recv().await.expect("receive inline item");
    assert_eq!(inline_item.mime_type, DEFAULT_TEXT_MIME);
    assert_eq!(inline_item.data, b"482910");
    assert!(inline_item.is_otp);

    let blob_payload = vec![0xCD; MAX_INLINE_CLIPBOARD_BYTES + 2048];
    sender_engine
        .send_clipboard_payload(
            &sender_node,
            "pixel-phone",
            "image/png",
            &blob_payload,
            false,
        )
        .await
        .expect("send large clipboard blob");
    let blob_item = rx.recv().await.expect("receive blob item");
    assert_eq!(blob_item.mime_type, "image/png");
    assert_eq!(blob_item.data, blob_payload);
    assert!(!blob_item.is_otp);

    let err = sender_engine
        .send_clipboard_text(&sender_node, "pixel-phone", "", false)
        .await
        .expect_err("empty text must fail");
    assert!(matches!(
        err,
        KeynearError::Validation { field: "text", .. }
    ));

    receiver_task.abort();
    sender_node.close().await;
    receiver_node.close().await;
}

#[test]
fn delayed_offer_validation_render_selection_and_eviction() {
    let engine = ClipboardEngine::new(ClipboardPolicy::default());

    assert!(matches!(
        engine.register_delayed_offer(vec![], false, false, 0, Arc::new(|_| Ok(vec![1]))),
        Err(KeynearError::Validation {
            field: "mimeTypes",
            ..
        })
    ));
    let too_many_mimes: Vec<String> = (0..=history::MAX_OFFER_MIME_COUNT)
        .map(|i| format!("text/x-item-{i}"))
        .collect();
    assert!(matches!(
        engine.register_delayed_offer(too_many_mimes, false, false, 0, Arc::new(|_| Ok(vec![1])),),
        Err(KeynearError::Validation {
            field: "mimeTypes",
            ..
        })
    ));

    let text_offer = engine
        .register_delayed_offer(
            vec!["text/plain".to_string(), "text/html".to_string()],
            false,
            false,
            5,
            Arc::new(|mime| Ok(mime.as_bytes().to_vec())),
        )
        .expect("register text offer");
    let (chosen_mime, rendered, is_otp) = engine
        .render_outgoing_offer(&text_offer.offer_id, "")
        .expect("render default text mime");
    assert_eq!(chosen_mime, "text/plain");
    assert_eq!(rendered, b"text/plain");
    assert!(!is_otp);

    assert!(matches!(
        engine.render_outgoing_offer(&text_offer.offer_id, "image/png"),
        Err(KeynearError::Validation {
            field: "mimeType",
            ..
        })
    ));
    assert!(matches!(
        engine.render_outgoing_offer("missing-offer", ""),
        Err(KeynearError::Transfer(_))
    ));

    let img_offer = engine
        .register_delayed_offer(
            vec!["image/jpeg".to_string(), "image/png".to_string()],
            true,
            false,
            16,
            Arc::new(|mime| Ok(mime.as_bytes().to_vec())),
        )
        .expect("register image offer");
    let (img_mime, _, _) = engine
        .render_outgoing_offer(&img_offer.offer_id, "")
        .expect("render preferred image/png");
    assert_eq!(img_mime, "image/png");

    let empty_offer = engine
        .register_delayed_offer(
            vec!["text/plain".to_string()],
            false,
            false,
            0,
            Arc::new(|_| Ok(Vec::new())),
        )
        .expect("register empty callback offer");
    assert!(matches!(
        engine.render_outgoing_offer(&empty_offer.offer_id, ""),
        Err(KeynearError::Transfer(_))
    ));

    let oversized_offer = engine
        .register_delayed_offer(
            vec!["image/png".to_string()],
            true,
            false,
            (MAX_CLIPBOARD_IMAGE_BYTES + 1) as u64,
            Arc::new(|_| Ok(vec![0u8; MAX_CLIPBOARD_IMAGE_BYTES + 1])),
        )
        .expect("register oversized callback offer");
    assert!(matches!(
        engine.render_outgoing_offer(&oversized_offer.offer_id, ""),
        Err(KeynearError::FrameTooLarge { .. })
    ));

    let mut offer_ids = Vec::new();
    for idx in 0..=history::MAX_OUTGOING_OFFERS {
        let offer = engine
            .register_delayed_offer(
                vec![format!("text/x-slot-{idx}")],
                false,
                false,
                4,
                Arc::new(|_| Ok(b"data".to_vec())),
            )
            .expect("register eviction test offer");
        offer_ids.push(offer.offer_id);
    }
    assert!(engine.render_outgoing_offer(&offer_ids[0], "").is_err());
    assert!(engine
        .render_outgoing_offer(offer_ids.last().expect("last"), "")
        .is_ok());
}
