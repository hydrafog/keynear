use super::*;

use iroh::endpoint::SendStream;

use super::paths::zip_err;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_file_handle_stream(
    registry: &TransferRegistry,
    conn: &Connection,
    offer: &v1::TransferOffer,
    file: File,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<String, KeynearError> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|err| KeynearError::Transport(format!("open transfer stream failed: {err}")))?;

    write_stream_kind(&mut send, QuicStreamKind::RawFileChunk).await?;
    let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
    write_control_envelope(&mut send, &offer_env).await?;

    await_transfer_consent_with_cancel(&mut recv, &offer.transfer_id, cancel_flag).await?;

    let (chunk_tx, mut chunk_rx) =
        mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
    let reader_task = spawn_file_reader(file, Arc::clone(cancel_flag), chunk_tx);
    let mut ctx = SendContext {
        send: &mut send,
        registry,
        offer,
        cancel_flag,
    };
    let (bytes_sent, digest) = send_chunks(&mut ctx, &mut chunk_rx).await?;

    reader_task
        .await
        .map_err(|err| KeynearError::Transfer(format!("file reader task failed: {err}")))?;

    send_completion_and_wait_ack(
        &mut send,
        &mut recv,
        CompletionDetails {
            transfer_id: &offer.transfer_id,
            bytes_transferred: bytes_sent,
            digest: &digest,
        },
    )
    .await
    .map(|()| hex::encode(digest))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_zip_stream(
    registry: &TransferRegistry,
    conn: &Connection,
    offer: &v1::TransferOffer,
    entries: Vec<ArchiveSourceEntry>,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<String, KeynearError> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|err| KeynearError::Transport(format!("open zip stream failed: {err}")))?;

    write_stream_kind(&mut send, QuicStreamKind::StoredZipArchive).await?;
    let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
    write_control_envelope(&mut send, &offer_env).await?;

    await_transfer_consent_with_cancel(&mut recv, &offer.transfer_id, cancel_flag).await?;

    let (chunk_tx, mut chunk_rx) =
        mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
    let writer_task = spawn_zip_writer(entries, Arc::clone(cancel_flag), chunk_tx);
    let mut ctx = SendContext {
        send: &mut send,
        registry,
        offer,
        cancel_flag,
    };
    let (bytes_sent, digest) = send_chunks(&mut ctx, &mut chunk_rx).await?;

    let writer_res = writer_task
        .await
        .map_err(|err| KeynearError::Transfer(format!("zip writer task failed: {err}")))?;
    writer_res?;

    send_completion_and_wait_ack(
        &mut send,
        &mut recv,
        CompletionDetails {
            transfer_id: &offer.transfer_id,
            bytes_transferred: bytes_sent,
            digest: &digest,
        },
    )
    .await
    .map(|()| hex::encode(digest))
}

fn spawn_file_reader(
    file: File,
    cancel_reader: Arc<AtomicBool>,
    chunk_tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
        let mut buf = vec![0u8; STREAM_CHUNK_BYTES];
        loop {
            if cancel_reader.load(Ordering::Acquire) {
                let _ = chunk_tx.blocking_send(Err(KeynearError::TransferCancelled(
                    "cancelled by sender".to_string(),
                )));
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if chunk_tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = chunk_tx.blocking_send(Err(KeynearError::Io(err)));
                    break;
                }
            }
        }
    })
}

fn spawn_zip_writer(
    entries: Vec<ArchiveSourceEntry>,
    cancel_writer: Arc<AtomicBool>,
    chunk_tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
) -> tokio::task::JoinHandle<Result<(), KeynearError>> {
    tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
        let channel_writer = ChannelChunkWriter::new(chunk_tx.clone(), cancel_writer);
        let buffered = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, channel_writer);
        let mut zip = ZipWriter::new_stream(buffered);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        for entry in entries {
            if entry.is_dir {
                let dir_name = if entry.archive_path.ends_with('/') {
                    entry.archive_path
                } else {
                    format!("{}/", entry.archive_path)
                };
                zip.add_directory(dir_name, options).map_err(zip_err)?;
                continue;
            }
            let Some(src_path) = entry.source_path else {
                continue;
            };
            let meta = fs::symlink_metadata(&src_path)?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                continue;
            }
            zip.start_file(&entry.archive_path, options)
                .map_err(zip_err)?;
            let file = File::open(&src_path)?;
            let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
            let mut copy_buf = vec![0u8; STREAM_CHUNK_BYTES];
            loop {
                let n = reader.read(&mut copy_buf)?;
                if n == 0 {
                    break;
                }
                zip.write_all(&copy_buf[..n])?;
            }
        }

        let stream_writer = zip.finish().map_err(zip_err)?;
        let mut buffered = stream_writer.into_inner();
        buffered.flush()?;
        Ok(())
    })
}

struct SendContext<'a> {
    send: &'a mut SendStream,
    registry: &'a TransferRegistry,
    offer: &'a v1::TransferOffer,
    cancel_flag: &'a Arc<AtomicBool>,
}

async fn send_chunks(
    ctx: &mut SendContext<'_>,
    chunk_rx: &mut mpsc::Receiver<Result<Vec<u8>, KeynearError>>,
) -> Result<(u64, [u8; 32]), KeynearError> {
    let mut hasher = Sha256::new();
    let mut bytes_sent: u64 = 0;
    while let Some(item) = chunk_rx.recv().await {
        if ctx.cancel_flag.load(Ordering::Acquire) {
            let _ = send_cancel_frame(
                &mut *ctx.send,
                &ctx.offer.transfer_id,
                "cancelled_by_sender",
            )
            .await;
            return Err(KeynearError::TransferCancelled(
                "cancelled by sender".to_string(),
            ));
        }
        let chunk = item?;
        hasher.update(&chunk);
        write_data_chunk(&mut *ctx.send, &chunk).await?;
        bytes_sent = bytes_sent.saturating_add(chunk.len() as u64);
        ctx.registry
            .update_progress(&ctx.offer.transfer_id, bytes_sent);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    Ok((bytes_sent, digest))
}
