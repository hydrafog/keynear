use std::path::PathBuf;

use keynear_protocol::KeynearError;

fn validation(message: String) -> KeynearError {
    KeynearError::Validation {
        field: "uri",
        message,
    }
}

fn local_path_part(uri: &str) -> Result<String, KeynearError> {
    let after_scheme = uri
        .strip_prefix("file://")
        .ok_or_else(|| validation(format!("expected file:// URI, got '{uri}'")))?;
    if let Some(rest) = after_scheme.strip_prefix("localhost/") {
        Ok(format!("/{rest}"))
    } else if after_scheme.starts_with('/') {
        Ok(after_scheme.to_string())
    } else {
        Err(validation(format!(
            "unsupported non-local file URI '{uri}'"
        )))
    }
}

fn percent_decode(bytes: &[u8]) -> Result<Vec<u8>, KeynearError> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(validation(
                    "truncated percent-encoding in file URI".to_string(),
                ));
            }
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            match (hi, lo) {
                (Some(h), Some(l)) => {
                    decoded.push(((h << 4) | l) as u8);
                    i += 3;
                }
                _ => {
                    return Err(validation("invalid hex escape in file URI".to_string()));
                }
            }
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    Ok(decoded)
}

pub fn file_uri_to_path(uri: &str) -> Result<PathBuf, KeynearError> {
    let raw_path = local_path_part(uri)?;
    let decoded = percent_decode(raw_path.as_bytes())?;

    if decoded.is_empty() || decoded.contains(&0) {
        return Err(validation(
            "file URI resolves to an empty or NUL path".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(decoded)
            .map(PathBuf::from)
            .map_err(|_| validation("file URI is not valid UTF-8".to_string()))
    }
}
