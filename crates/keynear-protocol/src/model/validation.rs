use super::*;

pub fn validate_display_name(raw: &str) -> Result<String, KeynearError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(KeynearError::Validation {
            field: "displayName",
            message: "display_name must not be empty".to_string(),
        });
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(KeynearError::Validation {
            field: "displayName",
            message: "display_name must not contain control characters".to_string(),
        });
    }
    if trimmed.chars().count() > MAX_DISPLAY_NAME_LEN {
        return Err(KeynearError::Validation {
            field: "displayName",
            message: format!("display_name must not exceed {MAX_DISPLAY_NAME_LEN} characters"),
        });
    }
    Ok(trimmed.to_string())
}

pub fn sanitize_device_id(raw: &str) -> Result<String, KeynearError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_DEVICE_ID_BYTES {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "device_id must be between 1 and 64 bytes".to_string(),
        });
    }
    if trimmed.contains("..")
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "device_id contains invalid characters".to_string(),
        });
    }
    Ok(trimmed.to_string())
}
