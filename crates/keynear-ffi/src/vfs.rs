use std::io::Write;

use keynear_core::{
    read_plan, resume_from_checkpoint, CheckpointInput, ChunkBitmap, RemoteVfsNode, ResumeDecision,
    CAP_RESUMABLE_TRANSFER, STREAM_CHUNK_BYTES,
};

use crate::callbacks::FfiError;
use crate::transfer::wrap_raw_fd;

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
pub fn transfer_resume_decision(
    local_flags: u32,
    peer_flags: u32,
    checkpoint: FfiChunkCheckpoint,
    last_sequence: u64,
) -> Result<FfiResumeDecision, FfiError> {
    let bitmap = ChunkBitmap::new(checkpoint.chunk_count, checkpoint.chunk_bitmap)?;
    let input = CheckpointInput {
        local_flags,
        peer_flags,
        transfer_id: &checkpoint.transfer_id,
        total_bytes: checkpoint
            .chunk_count
            .saturating_mul(STREAM_CHUNK_BYTES as u64),
        sequence: checkpoint.sequence,
        last_sequence,
        verified_bytes: checkpoint.verified_bytes,
        bitmap: &bitmap,
        rolling_digest: &checkpoint.rolling_sha256,
    };
    let decision = resume_from_checkpoint(input)?;
    Ok(decision.into())
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

#[uniffi::export]
pub fn provide_vfs_read_fd(raw_fd: i32, chunks: Vec<Vec<u8>>) -> Result<u64, FfiError> {
    let owned = wrap_raw_fd(raw_fd)?;
    let mut file = std::fs::File::from(owned);
    let mut written = 0u64;
    for chunk in chunks {
        if chunk.len() > STREAM_CHUNK_BYTES {
            return Err(FfiError::FrameTooLarge {
                size: chunk.len() as u64,
                max: STREAM_CHUNK_BYTES as u64,
            });
        }
        file.write_all(&chunk)
            .map_err(keynear_core::KeynearError::Io)?;
        written += chunk.len() as u64;
    }
    Ok(written)
}
