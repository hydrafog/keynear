use sha2::{Digest, Sha256};

use keynear_protocol::{KeynearError, STREAM_CHUNK_BYTES};

use crate::capability::{negotiate_capabilities, CAP_RESUMABLE_TRANSFER};

pub const MAX_CHECKPOINT_BITMAP_BYTES: usize = 16_384;
pub const MAX_TRANSFER_ID_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeDecision {
    Full,
    FromChunk(u64),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkBitmap {
    bytes: Vec<u8>,
    verified_chunks: u64,
}

impl ChunkBitmap {
    pub fn new(chunk_count: u64, bytes: Vec<u8>) -> Result<Self, KeynearError> {
        let expected_len = chunk_count.div_ceil(8) as usize;
        if chunk_count == 0 || expected_len > MAX_CHECKPOINT_BITMAP_BYTES {
            return Err(KeynearError::Validation {
                field: "chunkBitmap",
                message: "checkpoint chunk count is out of bounds".to_string(),
            });
        }
        if bytes.len() != expected_len {
            return Err(KeynearError::Validation {
                field: "chunkBitmap",
                message: "checkpoint bitmap length does not match chunk count".to_string(),
            });
        }
        if let Some(last) = bytes.last() {
            let used_bits = (chunk_count % 8) as u8;
            if used_bits != 0 && last & !((1u8 << used_bits) - 1) != 0 {
                return Err(KeynearError::Validation {
                    field: "chunkBitmap",
                    message: "checkpoint bitmap sets bits beyond the chunk count".to_string(),
                });
            }
        }

        let mut verified_chunks = 0u64;
        let mut seen_gap = false;
        for index in 0..chunk_count {
            let byte = bytes[(index / 8) as usize];
            let bit = (byte >> (index % 8)) & 1;
            if bit == 0 {
                seen_gap = true;
            } else if seen_gap {
                return Err(KeynearError::Validation {
                    field: "chunkBitmap",
                    message: "checkpoint bitmap must mark a contiguous verified prefix".to_string(),
                });
            } else {
                verified_chunks += 1;
            }
        }
        Ok(Self {
            bytes,
            verified_chunks,
        })
    }

    pub fn verified_chunks(&self) -> u64 {
        self.verified_chunks
    }

    pub fn is_verified(&self, index: u64) -> bool {
        let byte = match self.bytes.get((index / 8) as usize) {
            Some(value) => *value,
            None => return false,
        };
        (byte >> (index % 8)) & 1 == 1
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone)]
pub struct Sha256Chain {
    hasher: Sha256,
    chunks: u64,
}

impl Default for Sha256Chain {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256Chain {
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            chunks: 0,
        }
    }

    pub fn extend(&mut self, chunk: &[u8]) -> Result<(), KeynearError> {
        if chunk.is_empty() || chunk.len() > STREAM_CHUNK_BYTES {
            return Err(KeynearError::Transfer(
                "rolling digest chunk length out of bounds".to_string(),
            ));
        }
        self.hasher.update(chunk);
        self.chunks += 1;
        Ok(())
    }

    pub fn chunks(&self) -> u64 {
        self.chunks
    }

    pub fn digest(&self) -> [u8; 32] {
        let clone = self.hasher.clone();
        clone.finalize().into()
    }

    pub fn finalize(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

fn validate_transfer_id(transfer_id: &str) -> Result<(), KeynearError> {
    if transfer_id.trim().is_empty()
        || transfer_id.len() > MAX_TRANSFER_ID_LEN
        || transfer_id.chars().any(char::is_control)
    {
        return Err(KeynearError::Validation {
            field: "transferId",
            message: "transfer_id must be 1..64 safe characters".to_string(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CheckpointInput<'a> {
    pub local_flags: u32,
    pub peer_flags: u32,
    pub transfer_id: &'a str,
    pub total_bytes: u64,
    pub sequence: u64,
    pub last_sequence: u64,
    pub verified_bytes: u64,
    pub bitmap: &'a ChunkBitmap,
    pub rolling_digest: &'a [u8],
}

pub fn resume_from_checkpoint(input: CheckpointInput<'_>) -> Result<ResumeDecision, KeynearError> {
    validate_transfer_id(input.transfer_id)?;
    if input.total_bytes == 0 {
        return Err(KeynearError::Validation {
            field: "totalBytes",
            message: "total_bytes must be positive".to_string(),
        });
    }
    if input.rolling_digest.len() != 32 {
        return Err(KeynearError::Validation {
            field: "rollingSha256",
            message: "rolling digest must be 32 bytes".to_string(),
        });
    }

    let negotiated =
        negotiate_capabilities(input.local_flags, input.peer_flags) & CAP_RESUMABLE_TRANSFER;
    if negotiated == 0 || input.sequence <= input.last_sequence {
        return Ok(ResumeDecision::Unavailable);
    }

    let chunk_count = input.total_bytes.div_ceil(STREAM_CHUNK_BYTES as u64);
    let bitmap_chunks = input.bitmap.verified_chunks();
    if bitmap_chunks > chunk_count {
        return Err(KeynearError::Validation {
            field: "chunkBitmap",
            message: "checkpoint verifies more chunks than the offer contains".to_string(),
        });
    }
    let expected_bytes =
        (bitmap_chunks as u128 * STREAM_CHUNK_BYTES as u128).min(input.total_bytes as u128) as u64;
    if input.verified_bytes != expected_bytes {
        return Err(KeynearError::Validation {
            field: "verifiedBytes",
            message: "checkpoint verified bytes do not match the bitmap".to_string(),
        });
    }

    if bitmap_chunks == 0 {
        Ok(ResumeDecision::Full)
    } else {
        Ok(ResumeDecision::FromChunk(bitmap_chunks))
    }
}

#[cfg(test)]
#[path = "resume/tests.rs"]
mod tests;
