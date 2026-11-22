use std::collections::HashMap;

use keynear_protocol::KeynearError;

use crate::capability::{negotiate_capabilities, CAP_NOTIFICATION_MIRROR};

pub const MAX_NOTIFICATION_KEY_LEN: usize = 128;
pub const MAX_PACKAGE_NAME_LEN: usize = 128;
pub const MAX_NOTIFICATION_TITLE_LEN: usize = 256;
pub const MAX_NOTIFICATION_BODY_LEN: usize = 2_048;
pub const MAX_CATEGORY_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDescriptor<'a> {
    pub key: &'a str,
    pub package: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub category: &'a str,
    pub sequence: u64,
    pub supports_reply: bool,
    pub supports_mark_read: bool,
    pub supports_dismiss: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirroredNotificationView {
    pub notification_key: String,
    pub package_name: String,
    pub title: String,
    pub body: String,
    pub category: String,
    pub sequence: u64,
    pub supports_reply: bool,
    pub supports_mark_read: bool,
    pub supports_dismiss: bool,
}

#[derive(Debug, Default)]
pub struct MirrorEngine {
    last_sequences: HashMap<String, u64>,
}

impl MirrorEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(
        &mut self,
        descriptor: NotificationDescriptor<'_>,
        exclusions: &[String],
        local_flags: u32,
        peer_flags: u32,
    ) -> Result<Option<MirroredNotificationView>, KeynearError> {
        if negotiate_capabilities(local_flags, peer_flags) & CAP_NOTIFICATION_MIRROR == 0 {
            return Ok(None);
        }
        validate_descriptor(&descriptor)?;
        if is_package_excluded(descriptor.package, exclusions) {
            return Ok(None);
        }
        if self
            .last_sequences
            .get(descriptor.key)
            .is_some_and(|last| descriptor.sequence <= *last)
        {
            return Ok(None);
        }
        self.last_sequences
            .insert(descriptor.key.to_string(), descriptor.sequence);
        Ok(Some(MirroredNotificationView {
            notification_key: descriptor.key.to_string(),
            package_name: descriptor.package.to_string(),
            title: descriptor.title.to_string(),
            body: descriptor.body.to_string(),
            category: descriptor.category.to_string(),
            sequence: descriptor.sequence,
            supports_reply: descriptor.supports_reply,
            supports_mark_read: descriptor.supports_mark_read,
            supports_dismiss: descriptor.supports_dismiss,
        }))
    }

    pub fn forget(&mut self, key: &str) {
        self.last_sequences.remove(key);
    }

    pub fn clear(&mut self) {
        self.last_sequences.clear();
    }
}

pub fn is_package_excluded(package: &str, exclusions: &[String]) -> bool {
    exclusions.iter().any(|excluded| excluded == package)
}

pub fn validate_descriptor(descriptor: &NotificationDescriptor<'_>) -> Result<(), KeynearError> {
    validate_text("notification_key", descriptor.key, MAX_NOTIFICATION_KEY_LEN)?;
    validate_text("package_name", descriptor.package, MAX_PACKAGE_NAME_LEN)?;
    validate_text("title", descriptor.title, MAX_NOTIFICATION_TITLE_LEN)?;
    validate_text("body", descriptor.body, MAX_NOTIFICATION_BODY_LEN)?;
    validate_text("category", descriptor.category, MAX_CATEGORY_LEN)
}

pub fn validate_reply_text(text: &str) -> Result<(), KeynearError> {
    if text.is_empty() {
        return Err(KeynearError::Validation {
            field: "reply_text",
            message: "inline reply must not be empty".to_string(),
        });
    }
    validate_text("reply_text", text, MAX_NOTIFICATION_BODY_LEN)
}

fn validate_text(field: &'static str, value: &str, max: usize) -> Result<(), KeynearError> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(KeynearError::Validation {
            field,
            message: format!("must be 1..{max} safe characters"),
        });
    }
    Ok(())
}
