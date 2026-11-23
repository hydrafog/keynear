use super::paths::{open_private_new_file, unique_target_path, zip_err};
use super::receive_common::ReceiveContext;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn receive_zip_stream(
    registry: &TransferRegistry,
    dir: PathBuf,
    offer: &v1::TransferOffer,
    recv: &mut RecvStream,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(u64, Vec<String>, String), KeynearError> {
    fs::create_dir_all(&dir)?;
    let staging_dir = dir.join(format!(
        ".keynear-staging-{}-{}",
        offer.transfer_id,
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&staging_dir)?;

    let mut ctx = ReceiveContext {
        registry,
        offer,
        recv,
        cancel_flag,
    };
    let archive_tmp_path = staging_dir.join(".archive.zip.tmp");
    let max_wire_bytes = offer
        .total_bytes
        .saturating_add(MAX_ZIP_STREAM_OVERHEAD_BYTES);
    let max_extracted = offer.total_bytes;
    let staging_for_extract = staging_dir.clone();

    let stream_result: Result<(u64, [u8; 32], Vec<PathBuf>), KeynearError> = async {
        let (wire_bytes, digest) =
            write_archive_and_verify(&mut ctx, &archive_tmp_path, max_wire_bytes).await?;
        let extracted_paths =
            extract_archive(&archive_tmp_path, &staging_for_extract, max_extracted).await?;
        let _ = fs::remove_file(&archive_tmp_path);
        Ok((wire_bytes, digest, extracted_paths))
    }
    .await;

    match stream_result {
        Ok((wire_bytes, digest, extracted_paths)) => {
            let promoted = finalize_zip_receive(&dir, &staging_dir, extracted_paths, offer)?;
            Ok((wire_bytes, promoted, hex::encode(digest)))
        }
        Err(err) => {
            let _ = fs::remove_dir_all(&staging_dir);
            Err(err)
        }
    }
}

async fn write_archive_and_verify(
    ctx: &mut ReceiveContext<'_>,
    archive_tmp_path: &Path,
    max_wire_bytes: u64,
) -> Result<(u64, [u8; 32]), KeynearError> {
    let registry = ctx.registry;
    let offer = ctx.offer;
    let recv = &mut *ctx.recv;
    let cancel_flag = ctx.cancel_flag;
    let mut archive_writer = BufWriter::with_capacity(
        ZIP_IO_BUFFER_BYTES,
        open_private_new_file(archive_tmp_path)?,
    );
    let mut hasher = Sha256::new();
    let mut wire_bytes: u64 = 0;

    while let Some(chunk) = read_data_chunk(recv).await? {
        if cancel_flag.load(Ordering::Acquire) {
            return Err(KeynearError::TransferCancelled(
                "cancelled by receiver".to_string(),
            ));
        }
        wire_bytes = wire_bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| KeynearError::Transfer("wire byte overflow".to_string()))?;
        if wire_bytes > max_wire_bytes {
            return Err(KeynearError::Transfer(
                "zip stream exceeded permitted framing overhead".to_string(),
            ));
        }
        hasher.update(&chunk);
        archive_writer.write_all(&chunk)?;
        registry.update_progress(&offer.transfer_id, wire_bytes.min(offer.total_bytes));
    }
    archive_writer.flush()?;
    drop(archive_writer);

    let digest: [u8; 32] = hasher.finalize().into();
    verify_completion_envelope(recv, &offer.transfer_id, wire_bytes, &digest).await?;
    Ok((wire_bytes, digest))
}

async fn extract_archive(
    archive_path: &Path,
    staging_dir: &Path,
    max_extracted: u64,
) -> Result<Vec<PathBuf>, KeynearError> {
    let archive_path_clone = archive_path.to_path_buf();
    let staging_clone = staging_dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>, KeynearError> {
        let archive_file = File::open(&archive_path_clone)?;
        let reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, archive_file);
        let mut archive = ZipArchive::new(reader).map_err(zip_err)?;
        if archive.len() > MAX_FILES_IN_ARCHIVE {
            return Err(KeynearError::Transfer(
                "archive exceeds maximum file entry count".to_string(),
            ));
        }
        let mut state = ExtractState {
            staging_dir: &staging_clone,
            buf: vec![0u8; STREAM_CHUNK_BYTES],
            extracted_bytes: 0,
            max_extracted,
        };
        let mut top_level_entries = Vec::new();
        for idx in 0..archive.len() {
            let mut entry = archive.by_index(idx).map_err(zip_err)?;
            let safe_rel = extract_one_entry(&mut entry, &mut state)?;
            record_top_level(&mut top_level_entries, &safe_rel);
        }
        Ok(top_level_entries)
    })
    .await
    .map_err(|err| KeynearError::Transfer(format!("zip extract task failed: {err}")))?
}

fn record_top_level(top_entries: &mut Vec<PathBuf>, safe_rel: &Path) {
    if let Some(Component::Normal(first)) = safe_rel.components().next() {
        let top = PathBuf::from(first);
        if !top_entries.contains(&top) {
            top_entries.push(top);
        }
    }
}

struct ExtractState<'a> {
    staging_dir: &'a Path,
    buf: Vec<u8>,
    extracted_bytes: u64,
    max_extracted: u64,
}

fn extract_one_entry<R: Read>(
    entry: &mut zip::read::ZipFile<'_, R>,
    state: &mut ExtractState<'_>,
) -> Result<PathBuf, KeynearError> {
    validate_zip_entry(entry)?;
    let Some(enclosed) = entry.enclosed_name() else {
        return Err(KeynearError::Transfer(
            "zip entry contains invalid or escaping path".to_string(),
        ));
    };
    let safe_rel = sanitize_relative_path(&enclosed.to_string_lossy())?;
    if entry.is_dir() {
        fs::create_dir_all(state.staging_dir.join(&safe_rel))?;
        return Ok(safe_rel);
    }
    if entry.compressed_size() != entry.size() {
        return Err(KeynearError::Transfer(
            "STORED zip entry has mismatched compressed and uncompressed size".to_string(),
        ));
    }
    let target = state.staging_dir.join(&safe_rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let out_file = open_private_new_file(&target)?;
    let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
    loop {
        let n = entry.read(&mut state.buf)?;
        if n == 0 {
            break;
        }
        state.extracted_bytes = state
            .extracted_bytes
            .checked_add(n as u64)
            .ok_or_else(|| KeynearError::Transfer("extracted byte overflow".to_string()))?;
        if state.extracted_bytes > state.max_extracted {
            return Err(KeynearError::Transfer(format!(
                "extracted bytes {} exceed TransferOffer.total_bytes {}",
                state.extracted_bytes, state.max_extracted
            )));
        }
        writer.write_all(&state.buf[..n])?;
    }
    writer.flush()?;
    Ok(safe_rel)
}

fn order_top_entries(top_entries: Vec<PathBuf>, offer: &v1::TransferOffer) -> Vec<PathBuf> {
    let mut ordered = Vec::with_capacity(top_entries.len());
    for item in &offer.items {
        if let Ok(clean) = sanitize_relative_path(&item.name) {
            if let Some(Component::Normal(first)) = clean.components().next() {
                let top = PathBuf::from(first);
                if top_entries.contains(&top) && !ordered.contains(&top) {
                    ordered.push(top);
                }
            }
        }
    }
    for entry in top_entries {
        if !ordered.contains(&entry) {
            ordered.push(entry);
        }
    }
    ordered
}

fn finalize_zip_receive(
    dir: &Path,
    staging_dir: &Path,
    top_entries: Vec<PathBuf>,
    offer: &v1::TransferOffer,
) -> Result<Vec<String>, KeynearError> {
    let ordered = order_top_entries(top_entries, offer);
    let mut promoted = Vec::with_capacity(ordered.len());
    for top in ordered {
        let src = staging_dir.join(&top);
        if !src.exists() {
            continue;
        }
        let dest = unique_target_path(dir, &top, src.is_dir());
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dest)?;
        promoted.push(dest.to_string_lossy().to_string());
    }
    let _ = fs::remove_dir_all(staging_dir);
    Ok(promoted)
}

fn validate_zip_entry<R: Read>(entry: &zip::read::ZipFile<'_, R>) -> Result<(), KeynearError> {
    if entry.encrypted() {
        return Err(KeynearError::Transfer(
            "encrypted zip entries are not allowed".to_string(),
        ));
    }
    if entry.is_symlink() {
        return Err(KeynearError::Transfer(
            "symlink zip entries are not allowed".to_string(),
        ));
    }
    if entry.compression() != CompressionMethod::Stored {
        return Err(KeynearError::Transfer(format!(
            "unsupported zip compression method {:?}; only STORED is permitted",
            entry.compression()
        )));
    }
    Ok(())
}
