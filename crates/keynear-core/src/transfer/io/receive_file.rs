use super::paths::{open_private_new_file, unique_target_path, unique_temp_path};
use super::receive_common::ReceiveContext;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn receive_raw_file_stream(
    registry: &TransferRegistry,
    incoming_dir: PathBuf,
    offer: &v1::TransferOffer,
    recv: &mut RecvStream,
    explicit_file: Option<File>,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(u64, Vec<String>, String), KeynearError> {
    let rel_name = sanitize_relative_path(&offer.items[0].name)?;
    let (dest_dir, tmp_path, final_path, out_file) = if let Some(file) = explicit_file {
        (None, None, None, file)
    } else {
        fs::create_dir_all(&incoming_dir)?;
        let final_target = unique_target_path(&incoming_dir, &rel_name, false);
        let tmp = unique_temp_path(&incoming_dir, &offer.transfer_id);
        if let Some(parent) = tmp.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = open_private_new_file(&tmp)?;
        (Some(incoming_dir), Some(tmp), Some(final_target), file)
    };

    let (chunk_tx, writer_task) = spawn_disk_writer(out_file);
    let mut ctx = ReceiveContext {
        registry,
        offer,
        recv,
        cancel_flag,
    };
    let stream_result = receive_raw_chunks(&mut ctx, chunk_tx, writer_task).await;
    match stream_result {
        Ok((total_received, digest)) => {
            let target_paths = finalize_raw_receive(tmp_path, final_path)?;
            let _ = dest_dir;
            Ok((total_received, target_paths, hex::encode(digest)))
        }
        Err(err) => {
            if let Some(tmp) = tmp_path {
                let _ = fs::remove_file(tmp);
            }
            Err(err)
        }
    }
}

fn spawn_disk_writer(
    out_file: File,
) -> (
    mpsc::Sender<Vec<u8>>,
    tokio::task::JoinHandle<Result<(), KeynearError>>,
) {
    let (chunk_tx, mut chunk_rx) = mpsc::channel::<Vec<u8>>(PIPE_CHANNEL_SLOTS);
    let writer_task = tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
        let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
        while let Some(chunk) = chunk_rx.blocking_recv() {
            writer.write_all(&chunk)?;
        }
        writer.flush()?;
        let inner = writer
            .into_inner()
            .map_err(|err| KeynearError::Io(err.into_error()))?;
        let _ = inner.sync_all();
        Ok(())
    });
    (chunk_tx, writer_task)
}

async fn receive_raw_chunks(
    ctx: &mut ReceiveContext<'_>,
    chunk_tx: mpsc::Sender<Vec<u8>>,
    writer_task: tokio::task::JoinHandle<Result<(), KeynearError>>,
) -> Result<(u64, [u8; 32]), KeynearError> {
    let registry = ctx.registry;
    let offer = ctx.offer;
    let recv = &mut *ctx.recv;
    let cancel_flag = ctx.cancel_flag;
    let mut hasher = Sha256::new();
    let mut total_received: u64 = 0;

    while let Some(chunk) = read_data_chunk(recv).await? {
        if cancel_flag.load(Ordering::Acquire) {
            return Err(KeynearError::TransferCancelled(
                "cancelled by receiver".to_string(),
            ));
        }
        let next_total = total_received
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| KeynearError::Transfer("byte count overflow".to_string()))?;
        if next_total > offer.total_bytes {
            return Err(KeynearError::Transfer(format!(
                "received bytes {next_total} exceed TransferOffer.total_bytes {}",
                offer.total_bytes
            )));
        }
        hasher.update(&chunk);
        if chunk_tx.send(chunk).await.is_err() {
            return Err(KeynearError::Transfer(
                "disk writer task terminated unexpectedly".to_string(),
            ));
        }
        total_received = next_total;
        registry.update_progress(&offer.transfer_id, total_received);
    }

    drop(chunk_tx);
    let write_res = writer_task
        .await
        .map_err(|err| KeynearError::Transfer(format!("disk writer join failed: {err}")))?;
    write_res?;

    let digest: [u8; 32] = hasher.finalize().into();
    verify_completion_envelope(recv, &offer.transfer_id, total_received, &digest).await?;
    Ok((total_received, digest))
}

fn finalize_raw_receive(
    tmp_path: Option<PathBuf>,
    final_path: Option<PathBuf>,
) -> Result<Vec<String>, KeynearError> {
    if let (Some(tmp), Some(final_target)) = (tmp_path, final_path) {
        if let Some(parent) = final_target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&tmp, &final_target)?;
        Ok(vec![final_target.to_string_lossy().to_string()])
    } else {
        Ok(vec!["mediastore://fd".to_string()])
    }
}
