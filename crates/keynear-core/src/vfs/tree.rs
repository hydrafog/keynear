use keynear_protocol::KeynearError;

use crate::transfer::sanitize_relative_path;

pub const MAX_VFS_NAME_LEN: usize = 128;
pub const MAX_DOCUMENT_ID_LEN: usize = 256;
pub const VFS_PAGE_SIZE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteVfsNode {
    pub document_id: String,
    pub name: String,
    pub is_directory: bool,
    pub size_bytes: u64,
}

fn validate_name(raw: &str, field: &'static str) -> Result<(), KeynearError> {
    if raw.trim().is_empty() || raw.len() > MAX_VFS_NAME_LEN || raw.chars().any(char::is_control) {
        return Err(KeynearError::Validation {
            field,
            message: "name must be 1..128 safe characters".to_string(),
        });
    }
    Ok(())
}

impl RemoteVfsNode {
    pub fn root(root_id: &str, label: &str) -> Result<Self, KeynearError> {
        validate_name(root_id, "rootId")?;
        validate_name(label, "name")?;
        Ok(Self {
            document_id: root_id.to_string(),
            name: label.to_string(),
            is_directory: true,
            size_bytes: 0,
        })
    }

    pub fn child(
        root_id: &str,
        relative: &str,
        is_directory: bool,
        size_bytes: u64,
    ) -> Result<Self, KeynearError> {
        validate_name(root_id, "rootId")?;
        validate_name(relative, "name")?;
        let clean = sanitize_relative_path(relative)?;
        let document_id = format!("{}/{}", root_id, clean.to_string_lossy());
        if document_id.len() > MAX_DOCUMENT_ID_LEN {
            return Err(KeynearError::Validation {
                field: "documentId",
                message: "document identifier exceeds the bound".to_string(),
            });
        }
        Ok(Self {
            document_id,
            name: relative.to_string(),
            is_directory,
            size_bytes,
        })
    }
}

pub fn page_documents(nodes: &[RemoteVfsNode], offset: usize, limit: usize) -> &[RemoteVfsNode] {
    if limit == 0 {
        return &[];
    }
    let start = offset.min(nodes.len());
    let end = (start + limit).min(nodes.len());
    &nodes[start..end]
}
