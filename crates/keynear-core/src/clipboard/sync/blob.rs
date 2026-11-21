use super::*;

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
