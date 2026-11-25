use keynear_core::{
    read_plan, RemoteVfsNode, ResumeDecision, CAP_RESUMABLE_TRANSFER, STREAM_CHUNK_BYTES,
};

use crate::callbacks::FfiError;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FfiResumeDecision {
    Full,
    FromChunk { chunk: u64 },
    Unavailable,
}

impl From<ResumeDecision> for FfiResumeDecision {
    fn from(value: ResumeDecision) -> Self {
        match value {
            ResumeDecision::Full => Self::Full,
            ResumeDecision::FromChunk(chunk) => Self::FromChunk { chunk },
            ResumeDecision::Unavailable => Self::Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiChunkCheckpoint {
    pub transfer_id: String,
    pub sequence: u64,
    pub chunk_count: u64,
    pub chunk_bitmap: Vec<u8>,
    pub rolling_sha256: Vec<u8>,
    pub verified_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiReadPlan {
    pub offset: u64,
    pub length: u64,
    pub chunks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiVfsNode {
    pub document_id: String,
    pub name: String,
    pub is_directory: bool,
    pub size_bytes: u64,
}

impl From<RemoteVfsNode> for FfiVfsNode {
    fn from(value: RemoteVfsNode) -> Self {
        Self {
            document_id: value.document_id,
            name: value.name,
            is_directory: value.is_directory,
            size_bytes: value.size_bytes,
        }
    }
}

#[uniffi::export]
pub fn resumable_transfer_capability_flags() -> u32 {
    CAP_RESUMABLE_TRANSFER
}

#[uniffi::export]
pub fn vfs_read_plan(
    file: FfiVfsNode,
    offset: u64,
    max_bytes: u64,
) -> Result<FfiReadPlan, FfiError> {
    let max = usize::try_from(max_bytes).map_err(|_| FfiError::FrameTooLarge {
        size: max_bytes,
        max: STREAM_CHUNK_BYTES as u64,
    })?;
    let plan = read_plan(file.size_bytes, offset, max)?;
    Ok(FfiReadPlan {
        offset: plan.offset,
        length: plan.length as u64,
        chunks: plan.chunks,
    })
}

#[uniffi::export]
pub fn vfs_root_node(root_id: String, label: String) -> Result<FfiVfsNode, FfiError> {
    Ok(RemoteVfsNode::root(&root_id, &label)?.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(size: u64) -> FfiVfsNode {
        FfiVfsNode {
            document_id: "doc-1".to_string(),
            name: "file.txt".to_string(),
            is_directory: false,
            size_bytes: size,
        }
    }

    #[test]
    fn resume_decision_converts_all_variants() {
        assert_eq!(
            FfiResumeDecision::from(ResumeDecision::Full),
            FfiResumeDecision::Full
        );
        assert_eq!(
            FfiResumeDecision::from(ResumeDecision::FromChunk(7)),
            FfiResumeDecision::FromChunk { chunk: 7 }
        );
        assert_eq!(
            FfiResumeDecision::from(ResumeDecision::Unavailable),
            FfiResumeDecision::Unavailable
        );
    }

    #[test]
    fn node_conversion_and_root_builder() {
        let converted: FfiVfsNode = RemoteVfsNode::root("root", "Phone").expect("root").into();
        assert_eq!(converted.name, "Phone");
        assert!(converted.is_directory);
        assert_eq!(node(10).size_bytes, 10);
    }

    #[test]
    fn capability_flag_is_stable() {
        assert_eq!(
            resumable_transfer_capability_flags(),
            CAP_RESUMABLE_TRANSFER
        );
    }

    #[test]
    fn read_plan_clamps_offset_and_bounds_length() {
        let plan = vfs_read_plan(node(100), 90, 16).expect("plan");
        assert_eq!(plan.offset, 90);
        assert_eq!(plan.length, 10);
        assert_eq!(plan.chunks, 1);
        let empty = vfs_read_plan(node(0), 50, 16).expect("empty plan");
        assert_eq!(empty.length, 0);
        assert!(matches!(
            vfs_read_plan(node(10), 0, 0),
            Err(FfiError::Validation { .. })
        ));
    }

    #[test]
    fn root_builder_rejects_invalid_names() {
        assert!(matches!(
            vfs_root_node("root".to_string(), String::new()),
            Err(FfiError::Validation { .. })
        ));
    }
}
