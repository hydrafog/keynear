use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use iroh::endpoint::{RecvStream, SendStream};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use keynear_protocol::{
    read_control_envelope, read_stream_kind, v1, write_control_envelope, write_stream_kind,
    KeynearError, QuicStreamKind, STREAM_CHUNK_BYTES,
};

use crate::transport::OrionQuicNode;

use super::history::{
    check_policy, is_image_mime, normalize_mime_type, pick_preferred_image_mime,
    prune_incoming_offers, prune_outgoing_offers, validate_mime_type, ClipboardItem,
    ClipboardPolicy, DelayedClipboardOffer, DelayedRenderCallback, IncomingOfferEntry,
    OutgoingOfferEntry, DEFAULT_TEXT_MIME, MAX_CLIPBOARD_IMAGE_BYTES, MAX_CLIPBOARD_MIME_BYTES,
    MAX_CLIPBOARD_QUEUE, MAX_INCOMING_OFFERS, MAX_INLINE_CLIPBOARD_BYTES, MAX_OFFER_MIME_COUNT,
    MAX_OUTGOING_OFFERS,
};

pub async fn write_clipboard_blob<W: AsyncWrite + Unpin>(
    writer: &mut W,
    mime_type: &str,
    data: &[u8],
    is_otp: bool,
) -> Result<(), KeynearError> {
    let clean_mime = validate_mime_type(mime_type)?;
    if data.is_empty() {
        return Err(KeynearError::Validation {
            field: "data",
            message: "clipboard blob payload must not be empty".to_string(),
        });
    }
    if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: data.len(),
            max: MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }

    let digest = Sha256::digest(data);
    let mime_bytes = clean_mime.as_bytes();
    let mime_len = u16::try_from(mime_bytes.len()).map_err(|_| KeynearError::Validation {
        field: "mimeType",
        message: "MIME type length overflow".to_string(),
    })?;

    writer.write_all(&mime_len.to_be_bytes()).await?;
    writer.write_all(mime_bytes).await?;
    writer.write_all(&[u8::from(is_otp)]).await?;
    writer.write_all(&(data.len() as u64).to_be_bytes()).await?;
    writer.write_all(&digest).await?;

    for chunk in data.chunks(STREAM_CHUNK_BYTES) {
        writer.write_all(chunk).await?;
    }
    Ok(())
}

pub async fn read_clipboard_blob<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<(String, Vec<u8>, bool), KeynearError> {
    let mut mime_len_buf = [0u8; 2];
    reader.read_exact(&mut mime_len_buf).await?;
    let mime_len = u16::from_be_bytes(mime_len_buf) as usize;
    if mime_len == 0 || mime_len > MAX_CLIPBOARD_MIME_BYTES {
        return Err(KeynearError::Protocol(format!(
            "clipboard blob MIME length {mime_len} out of bounds"
        )));
    }
    let mut mime_bytes = vec![0u8; mime_len];
    reader.read_exact(&mut mime_bytes).await?;
    let mime_raw = String::from_utf8(mime_bytes).map_err(|_| {
        KeynearError::Protocol("clipboard blob MIME is not valid UTF-8".to_string())
    })?;
    let mime_type = validate_mime_type(&mime_raw)?;

    let mut otp_buf = [0u8; 1];
    reader.read_exact(&mut otp_buf).await?;
    let is_otp = otp_buf[0] != 0;

    let mut data_len_buf = [0u8; 8];
    reader.read_exact(&mut data_len_buf).await?;
    let data_len_u64 = u64::from_be_bytes(data_len_buf);
    let data_len = usize::try_from(data_len_u64).map_err(|_| KeynearError::FrameTooLarge {
        size: usize::MAX,
        max: MAX_CLIPBOARD_IMAGE_BYTES,
    })?;
    if data_len == 0 {
        return Err(KeynearError::Protocol(
            "clipboard blob payload must not be empty".to_string(),
        ));
    }
    if data_len > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: data_len,
            max: MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }

    let mut expected_sha256 = [0u8; 32];
    reader.read_exact(&mut expected_sha256).await?;

    let mut data = vec![0u8; data_len];
    reader.read_exact(&mut data).await?;

    let actual_sha256 = Sha256::digest(&data);
    if actual_sha256.as_slice() != expected_sha256 {
        return Err(KeynearError::Crypto(
            "clipboard blob SHA-256 digest mismatch".to_string(),
        ));
    }

    Ok((mime_type, data, is_otp))
}

pub struct ClipboardEngine {
    policy: RwLock<ClipboardPolicy>,
    outgoing_offers: Mutex<HashMap<String, OutgoingOfferEntry>>,
    incoming_offers: Mutex<HashMap<String, IncomingOfferEntry>>,
    incoming_queue: Mutex<VecDeque<ClipboardItem>>,
    incoming_tx: Mutex<Option<mpsc::UnboundedSender<ClipboardItem>>>,
    sequence: AtomicU64,
}

impl ClipboardEngine {
    pub fn new(policy: ClipboardPolicy) -> Self {
        Self {
            policy: RwLock::new(policy),
            outgoing_offers: Mutex::new(HashMap::new()),
            incoming_offers: Mutex::new(HashMap::new()),
            incoming_queue: Mutex::new(VecDeque::with_capacity(MAX_CLIPBOARD_QUEUE)),
            incoming_tx: Mutex::new(None),
            sequence: AtomicU64::new(1),
        }
    }

    pub fn with_incoming_sender(
        policy: ClipboardPolicy,
        tx: mpsc::UnboundedSender<ClipboardItem>,
    ) -> Self {
        let engine = Self::new(policy);
        if let Ok(mut guard) = engine.incoming_tx.lock() {
            *guard = Some(tx);
        }
        engine
    }

    pub fn set_policy(&self, policy: ClipboardPolicy) {
        let mut guard = self.policy.write().unwrap_or_else(|p| p.into_inner());
        *guard = policy;
    }

    pub fn policy(&self) -> ClipboardPolicy {
        self.policy
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn poll_incoming(&self) -> Vec<ClipboardItem> {
        let mut guard = self
            .incoming_queue
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        guard.drain(..).collect()
    }

    pub fn can_accept_mimes(&self, mimes: &[String], is_image: bool, is_otp: bool) -> bool {
        let policy = self.policy();
        check_policy(&policy, mimes, is_image, is_otp).is_ok()
    }

    pub fn register_delayed_offer(
        &self,
        mime_types: Vec<String>,
        is_image: bool,
        is_otp: bool,
        byte_size: u64,
        render: DelayedRenderCallback,
    ) -> Result<DelayedClipboardOffer, KeynearError> {
        if mime_types.is_empty() || mime_types.len() > MAX_OFFER_MIME_COUNT {
            return Err(KeynearError::Validation {
                field: "mimeTypes",
                message: format!("MIME list must contain 1..={MAX_OFFER_MIME_COUNT} entries"),
            });
        }
        let cleaned: Vec<String> = mime_types
            .iter()
            .map(|m| validate_mime_type(m))
            .collect::<Result<Vec<_>, _>>()?;

        let policy = self.policy();
        check_policy(&policy, &cleaned, is_image, is_otp)?;

        let offer_id = Uuid::new_v4().to_string();
        let entry = OutgoingOfferEntry {
            mime_types: cleaned.clone(),
            render,
            created_at: Instant::now(),
            is_image,
            is_otp,
        };

        {
            let mut offers = self
                .outgoing_offers
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            prune_outgoing_offers(&mut offers, Instant::now());
            if offers.len() >= MAX_OUTGOING_OFFERS {
                if let Some(oldest_key) = offers
                    .iter()
                    .min_by_key(|(_, v)| v.created_at)
                    .map(|(k, _)| k.clone())
                {
                    offers.remove(&oldest_key);
                }
            }
            offers.insert(offer_id.clone(), entry);
        }

        Ok(DelayedClipboardOffer {
            offer_id,
            mime_types: cleaned,
            byte_size,
            is_image,
            is_otp,
        })
    }

    pub fn render_outgoing_offer(
        &self,
        offer_id: &str,
        requested_mime: &str,
    ) -> Result<(String, Vec<u8>, bool), KeynearError> {
        let policy = self.policy();
        let (mime_types, render, is_image, is_otp) = {
            let mut offers = self
                .outgoing_offers
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            prune_outgoing_offers(&mut offers, Instant::now());
            let entry = offers.get(offer_id).ok_or_else(|| {
                KeynearError::Transfer("clipboard offer expired or unknown".to_string())
            })?;
            (
                entry.mime_types.clone(),
                Arc::clone(&entry.render),
                entry.is_image,
                entry.is_otp,
            )
        };

        check_policy(&policy, &mime_types, is_image, is_otp)?;

        let chosen_mime = if requested_mime.trim().is_empty() {
            if is_image {
                pick_preferred_image_mime(&mime_types)
                    .or_else(|| mime_types.first().cloned())
                    .ok_or_else(|| KeynearError::Validation {
                        field: "mimeType",
                        message: "no MIME type available for offer".to_string(),
                    })?
            } else {
                mime_types
                    .first()
                    .cloned()
                    .ok_or_else(|| KeynearError::Validation {
                        field: "mimeType",
                        message: "no MIME type available for offer".to_string(),
                    })?
            }
        } else {
            let req_norm = normalize_mime_type(requested_mime);
            mime_types
                .iter()
                .find(|m| normalize_mime_type(m) == req_norm)
                .cloned()
                .ok_or_else(|| KeynearError::Validation {
                    field: "mimeType",
                    message: format!("requested MIME '{requested_mime}' was not offered"),
                })?
        };

        check_policy(
            &policy,
            std::slice::from_ref(&chosen_mime),
            is_image,
            is_otp,
        )?;

        let data = render(&chosen_mime)?;
        if data.is_empty() {
            return Err(KeynearError::Transfer(
                "rendered clipboard payload is empty".to_string(),
            ));
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }

        Ok((chosen_mime, data, is_otp))
    }

    pub async fn send_clipboard_text(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        text: &str,
        is_otp: bool,
    ) -> Result<(), KeynearError> {
        if text.is_empty() {
            return Err(KeynearError::Validation {
                field: "text",
                message: "clipboard text must not be empty".to_string(),
            });
        }
        self.send_clipboard_payload(
            node,
            peer_device_id,
            DEFAULT_TEXT_MIME,
            text.as_bytes(),
            is_otp,
        )
        .await
    }

    pub async fn send_clipboard_payload(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        mime_type: &str,
        data: &[u8],
        is_otp: bool,
    ) -> Result<(), KeynearError> {
        let clean_mime = validate_mime_type(mime_type)?;
        let is_image = is_image_mime(&clean_mime);
        let policy = self.policy();
        check_policy(&policy, std::slice::from_ref(&clean_mime), is_image, is_otp)?;

        if data.is_empty() {
            return Err(KeynearError::Validation {
                field: "data",
                message: "clipboard payload must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(|err| {
            KeynearError::Transport(format!("open clipboard stream failed: {err}"))
        })?;

        if data.len() <= MAX_INLINE_CLIPBOARD_BYTES {
            write_stream_kind(&mut send, QuicStreamKind::Control).await?;
            let env = self.wrap_payload(v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
                mime_type: clean_mime,
                data: data.to_vec(),
                is_otp,
            }));
            write_control_envelope(&mut send, &env).await?;
        } else {
            write_stream_kind(&mut send, QuicStreamKind::ClipboardBlob).await?;
            write_clipboard_blob(&mut send, &clean_mime, data, is_otp).await?;
        }
        send.finish().map_err(|err| {
            KeynearError::Transport(format!("finish clipboard stream failed: {err}"))
        })?;

        await_control_ack(&mut recv).await
    }

    pub async fn send_delayed_offer(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        offer: &DelayedClipboardOffer,
    ) -> Result<(), KeynearError> {
        let policy = self.policy();
        check_policy(&policy, &offer.mime_types, offer.is_image, offer.is_otp)?;

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open offer stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let env = self.wrap_payload(v1::envelope::Payload::ClipboardOffer(v1::ClipboardOffer {
            offer_id: offer.offer_id.clone(),
            mime_types: offer.mime_types.clone(),
            is_image: offer.is_image,
            is_otp: offer.is_otp,
            byte_size: offer.byte_size,
        }));
        write_control_envelope(&mut send, &env).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish offer stream failed: {err}")))?;

        await_control_ack(&mut recv).await
    }

    pub async fn offer_clipboard_image(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        mime_types: Vec<String>,
        render: DelayedRenderCallback,
    ) -> Result<DelayedClipboardOffer, KeynearError> {
        let offer = self.register_delayed_offer(mime_types, true, false, 0, render)?;
        self.send_delayed_offer(node, peer_device_id, &offer)
            .await?;
        Ok(offer)
    }

    pub async fn fetch_clipboard_image(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        offer_id: &str,
        mime_type: &str,
    ) -> Result<ClipboardItem, KeynearError> {
        let clean_offer_id = offer_id.trim();
        if clean_offer_id.is_empty() {
            return Err(KeynearError::Validation {
                field: "offerId",
                message: "offer_id must not be empty".to_string(),
            });
        }
        let clean_mime = if mime_type.trim().is_empty() {
            String::new()
        } else {
            validate_mime_type(mime_type)?
        };

        let policy = self.policy();
        if !clean_mime.is_empty() {
            check_policy(
                &policy,
                std::slice::from_ref(&clean_mime),
                is_image_mime(&clean_mime),
                false,
            )?;
        } else if !policy.clipboard_sync_enabled || !policy.clipboard_images_enabled {
            return Err(KeynearError::Validation {
                field: "clipboardImagesEnabled",
                message: "clipboard image sync is disabled by policy".to_string(),
            });
        }

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open fetch stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let env = self.wrap_payload(v1::envelope::Payload::ClipboardFetch(v1::ClipboardFetch {
            offer_id: clean_offer_id.to_string(),
            mime_type: clean_mime,
        }));
        write_control_envelope(&mut send, &env).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish fetch stream failed: {err}")))?;

        let reply_kind = read_stream_kind(&mut recv).await?;
        let (resolved_mime, data, is_otp) = match reply_kind {
            QuicStreamKind::Control => {
                let reply = read_control_envelope(&mut recv).await?;
                match reply.payload {
                    Some(v1::envelope::Payload::Clipboard(payload)) => {
                        let valid_mime = validate_mime_type(&payload.mime_type)?;
                        if payload.data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
                            return Err(KeynearError::FrameTooLarge {
                                size: payload.data.len(),
                                max: MAX_CLIPBOARD_IMAGE_BYTES,
                            });
                        }
                        (valid_mime, payload.data, payload.is_otp)
                    }
                    Some(v1::envelope::Payload::ControlAck(ack)) if !ack.accepted => {
                        return Err(KeynearError::Transfer(ack.reason));
                    }
                    _ => {
                        return Err(KeynearError::Protocol(
                            "unexpected response payload for ClipboardFetch".to_string(),
                        ));
                    }
                }
            }
            QuicStreamKind::ClipboardBlob => read_clipboard_blob(&mut recv).await?,
            _ => {
                return Err(KeynearError::Protocol(
                    "unexpected stream kind in ClipboardFetch reply".to_string(),
                ));
            }
        };

        let policy_now = self.policy();
        check_policy(
            &policy_now,
            std::slice::from_ref(&resolved_mime),
            is_image_mime(&resolved_mime),
            is_otp,
        )?;

        Ok(ClipboardItem {
            id: clean_offer_id.to_string(),
            peer_device_id: peer_device_id.to_string(),
            mime_type: resolved_mime,
            data,
            is_otp,
            is_image_offer: false,
            offer_id: clean_offer_id.to_string(),
            offered_mimes: Vec::new(),
        })
    }

    pub async fn handle_incoming_stream_with_kind(
        &self,
        peer_device_id: &str,
        kind: QuicStreamKind,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<(), KeynearError> {
        match kind {
            QuicStreamKind::Control => {
                let env = read_control_envelope(&mut recv).await?;
                match env.payload {
                    Some(v1::envelope::Payload::Clipboard(payload)) => {
                        let policy = self.policy();
                        let valid_mime = match validate_mime_type(&payload.mime_type) {
                            Ok(m) => m,
                            Err(err) => {
                                send_control_ack(&mut send, env.sequence, false, &err.to_string())
                                    .await?;
                                return Err(err);
                            }
                        };
                        if let Err(err) = check_policy(
                            &policy,
                            std::slice::from_ref(&valid_mime),
                            is_image_mime(&valid_mime),
                            payload.is_otp,
                        ) {
                            send_control_ack(&mut send, env.sequence, false, &err.to_string())
                                .await?;
                            return Ok(());
                        }
                        if payload.data.is_empty() || payload.data.len() > MAX_CLIPBOARD_IMAGE_BYTES
                        {
                            send_control_ack(
                                &mut send,
                                env.sequence,
                                false,
                                "invalid clipboard payload size",
                            )
                            .await?;
                            return Err(KeynearError::FrameTooLarge {
                                size: payload.data.len(),
                                max: MAX_CLIPBOARD_IMAGE_BYTES,
                            });
                        }

                        let item = ClipboardItem {
                            id: Uuid::new_v4().to_string(),
                            peer_device_id: peer_device_id.to_string(),
                            mime_type: valid_mime,
                            data: payload.data,
                            is_otp: payload.is_otp,
                            is_image_offer: false,
                            offer_id: String::new(),
                            offered_mimes: Vec::new(),
                        };
                        self.enqueue_incoming(item);
                        send_control_ack(&mut send, env.sequence, true, "").await
                    }
                    Some(v1::envelope::Payload::ClipboardOffer(offer)) => {
                        if offer.offer_id.trim().is_empty()
                            || offer.mime_types.is_empty()
                            || offer.mime_types.len() > MAX_OFFER_MIME_COUNT
                        {
                            send_control_ack(
                                &mut send,
                                env.sequence,
                                false,
                                "invalid clipboard offer",
                            )
                            .await?;
                            return Err(KeynearError::Protocol(
                                "invalid ClipboardOffer metadata".to_string(),
                            ));
                        }
                        let mut clean_mimes = Vec::with_capacity(offer.mime_types.len());
                        for m in &offer.mime_types {
                            match validate_mime_type(m) {
                                Ok(clean) => clean_mimes.push(clean),
                                Err(err) => {
                                    send_control_ack(
                                        &mut send,
                                        env.sequence,
                                        false,
                                        &err.to_string(),
                                    )
                                    .await?;
                                    return Err(err);
                                }
                            }
                        }

                        let policy = self.policy();
                        if let Err(err) =
                            check_policy(&policy, &clean_mimes, offer.is_image, offer.is_otp)
                        {
                            send_control_ack(&mut send, env.sequence, false, &err.to_string())
                                .await?;
                            return Ok(());
                        }

                        let preferred_mime = pick_preferred_image_mime(&clean_mimes)
                            .or_else(|| clean_mimes.first().cloned())
                            .unwrap_or_else(|| DEFAULT_TEXT_MIME.to_string());

                        {
                            let mut offers = self
                                .incoming_offers
                                .lock()
                                .unwrap_or_else(|p| p.into_inner());
                            prune_incoming_offers(&mut offers, Instant::now());
                            if offers.len() >= MAX_INCOMING_OFFERS {
                                if let Some(oldest_key) = offers
                                    .iter()
                                    .min_by_key(|(_, v)| v.created_at)
                                    .map(|(k, _)| k.clone())
                                {
                                    offers.remove(&oldest_key);
                                }
                            }
                            offers.insert(
                                offer.offer_id.clone(),
                                IncomingOfferEntry {
                                    peer_device_id: peer_device_id.to_string(),
                                    mime_types: clean_mimes.clone(),
                                    is_image: offer.is_image,
                                    is_otp: offer.is_otp,
                                    created_at: Instant::now(),
                                },
                            );
                        }

                        let item = ClipboardItem {
                            id: offer.offer_id.clone(),
                            peer_device_id: peer_device_id.to_string(),
                            mime_type: preferred_mime,
                            data: Vec::new(),
                            is_otp: offer.is_otp,
                            is_image_offer: true,
                            offer_id: offer.offer_id,
                            offered_mimes: clean_mimes,
                        };
                        self.enqueue_incoming(item);
                        send_control_ack(&mut send, env.sequence, true, "").await
                    }
                    Some(v1::envelope::Payload::ClipboardFetch(fetch)) => {
                        let offer_id = fetch.offer_id;
                        let mime_type = fetch.mime_type;
                        let render_result = {
                            let policy = self.policy();
                            let found = {
                                let mut offers = self
                                    .outgoing_offers
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner());
                                prune_outgoing_offers(&mut offers, Instant::now());
                                offers.get(&offer_id).map(|e| {
                                    (
                                        e.mime_types.clone(),
                                        Arc::clone(&e.render),
                                        e.is_image,
                                        e.is_otp,
                                    )
                                })
                            };
                            match found {
                                None => Err(KeynearError::Transfer(
                                    "clipboard offer expired or unknown".to_string(),
                                )),
                                Some((mime_types, render, is_image, is_otp)) => {
                                    check_policy(&policy, &mime_types, is_image, is_otp)?;
                                    let chosen = if mime_type.trim().is_empty() {
                                        pick_preferred_image_mime(&mime_types)
                                            .or_else(|| mime_types.first().cloned())
                                    } else {
                                        let req_norm = normalize_mime_type(&mime_type);
                                        mime_types
                                            .iter()
                                            .find(|m| normalize_mime_type(m) == req_norm)
                                            .cloned()
                                    };
                                    match chosen {
                                        None => Err(KeynearError::Validation {
                                            field: "mimeType",
                                            message: "requested MIME type not offered".to_string(),
                                        }),
                                        Some(chosen_mime) => {
                                            check_policy(
                                                &policy,
                                                std::slice::from_ref(&chosen_mime),
                                                is_image,
                                                is_otp,
                                            )?;
                                            let data = tokio::task::spawn_blocking(move || {
                                                render(&chosen_mime)
                                                    .map(|bytes| (chosen_mime, bytes, is_otp))
                                            })
                                            .await
                                            .map_err(|err| {
                                                KeynearError::Transfer(format!(
                                                    "clipboard render task failed: {err}"
                                                ))
                                            })??;
                                            if data.1.is_empty() {
                                                Err(KeynearError::Transfer(
                                                    "rendered clipboard payload is empty"
                                                        .to_string(),
                                                ))
                                            } else if data.1.len() > MAX_CLIPBOARD_IMAGE_BYTES {
                                                Err(KeynearError::FrameTooLarge {
                                                    size: data.1.len(),
                                                    max: MAX_CLIPBOARD_IMAGE_BYTES,
                                                })
                                            } else {
                                                Ok(data)
                                            }
                                        }
                                    }
                                }
                            }
                        };

                        match render_result {
                            Ok((resolved_mime, data, is_otp)) => {
                                if data.len() <= MAX_INLINE_CLIPBOARD_BYTES {
                                    write_stream_kind(&mut send, QuicStreamKind::Control).await?;
                                    let reply_env = self.wrap_payload(
                                        v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
                                            mime_type: resolved_mime,
                                            data,
                                            is_otp,
                                        }),
                                    );
                                    write_control_envelope(&mut send, &reply_env).await?;
                                } else {
                                    write_stream_kind(&mut send, QuicStreamKind::ClipboardBlob)
                                        .await?;
                                    write_clipboard_blob(&mut send, &resolved_mime, &data, is_otp)
                                        .await?;
                                }
                                send.finish().map_err(|err| {
                                    KeynearError::Transport(format!(
                                        "finish fetch reply failed: {err}"
                                    ))
                                })?;
                                Ok(())
                            }
                            Err(err) => {
                                write_stream_kind(&mut send, QuicStreamKind::Control).await?;
                                send_control_ack(&mut send, env.sequence, false, &err.to_string())
                                    .await?;
                                Err(err)
                            }
                        }
                    }
                    _ => Err(KeynearError::Protocol(
                        "unexpected envelope payload on clipboard control stream".to_string(),
                    )),
                }
            }
            QuicStreamKind::ClipboardBlob => {
                let (mime_type, data, is_otp) = read_clipboard_blob(&mut recv).await?;
                let policy = self.policy();
                if let Err(err) = check_policy(
                    &policy,
                    std::slice::from_ref(&mime_type),
                    is_image_mime(&mime_type),
                    is_otp,
                ) {
                    send_control_ack(&mut send, 1, false, &err.to_string()).await?;
                    return Ok(());
                }
                let item = ClipboardItem {
                    id: Uuid::new_v4().to_string(),
                    peer_device_id: peer_device_id.to_string(),
                    mime_type,
                    data,
                    is_otp,
                    is_image_offer: false,
                    offer_id: String::new(),
                    offered_mimes: Vec::new(),
                };
                self.enqueue_incoming(item);
                send_control_ack(&mut send, 1, true, "").await
            }
            _ => Err(KeynearError::Protocol(
                "unexpected stream kind on clipboard stream".to_string(),
            )),
        }
    }

    pub fn lookup_incoming_offer(
        &self,
        offer_id: &str,
    ) -> Option<(String, Vec<String>, bool, bool)> {
        let mut offers = self
            .incoming_offers
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        prune_incoming_offers(&mut offers, Instant::now());
        offers.get(offer_id).map(|e| {
            (
                e.peer_device_id.clone(),
                e.mime_types.clone(),
                e.is_image,
                e.is_otp,
            )
        })
    }

    fn enqueue_incoming(&self, item: ClipboardItem) {
        if let Ok(guard) = self.incoming_tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(item);
                return;
            }
        }
        let mut queue = self
            .incoming_queue
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if queue.len() >= MAX_CLIPBOARD_QUEUE {
            queue.pop_front();
        }
        queue.push_back(item);
    }

    fn wrap_payload(&self, payload: v1::envelope::Payload) -> v1::Envelope {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        v1::Envelope {
            version: 1,
            sequence,
            timestamp_ms,
            payload: Some(payload),
        }
    }
}

async fn send_control_ack(
    send: &mut SendStream,
    acked_sequence: u64,
    accepted: bool,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = v1::Envelope {
        version: 1,
        sequence: acked_sequence,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
        payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence,
            accepted,
            reason: reason.to_string(),
        })),
    };
    write_control_envelope(send, &env).await?;
    send.finish()
        .map_err(|err| KeynearError::Transport(format!("finish control ack failed: {err}")))?;
    Ok(())
}

async fn await_control_ack(recv: &mut RecvStream) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    match env.payload {
        Some(v1::envelope::Payload::ControlAck(ack)) if ack.accepted => Ok(()),
        Some(v1::envelope::Payload::ControlAck(ack)) => Err(KeynearError::Transfer(ack.reason)),
        _ => Err(KeynearError::Protocol(
            "expected ControlAck on clipboard stream".to_string(),
        )),
    }
}
