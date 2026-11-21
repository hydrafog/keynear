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

pub fn resume_from_checkpoint(
    local_flags: u32,
    peer_flags: u32,
    transfer_id: &str,
    total_bytes: u64,
    sequence: u64,
    last_sequence: u64,
    verified_bytes: u64,
    bitmap: &ChunkBitmap,
    rolling_digest: &[u8],
) -> Result<ResumeDecision, KeynearError> {
    validate_transfer_id(transfer_id)?;
    if total_bytes == 0 {
        return Err(KeynearError::Validation {
            field: "totalBytes",
            message: "total_bytes must be positive".to_string(),
        });
    }
    if rolling_digest.len() != 32 {
        return Err(KeynearError::Validation {
            field: "rollingSha256",
            message: "rolling digest must be 32 bytes".to_string(),
        });
    }

    let negotiated = negotiate_capabilities(local_flags, peer_flags) & CAP_RESUMABLE_TRANSFER;
    if negotiated == 0 || sequence <= last_sequence {
        return Ok(ResumeDecision::Unavailable);
    }

    let chunk_count = total_bytes.div_ceil(STREAM_CHUNK_BYTES as u64);
    let bitmap_chunks = bitmap.verified_chunks();
    if bitmap_chunks > chunk_count {
        return Err(KeynearError::Validation {
            field: "chunkBitmap",
            message: "checkpoint verifies more chunks than the offer contains".to_string(),
        });
    }
    let expected_bytes =
        (bitmap_chunks as u128 * STREAM_CHUNK_BYTES as u128).min(total_bytes as u128) as u64;
    if verified_bytes != expected_bytes {
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
mod tests {
    use super::*;

    fn flags() -> (u32, u32) {
        (CAP_RESUMABLE_TRANSFER, CAP_RESUMABLE_TRANSFER)
    }

    fn bitmap(prefix: u64, count: u64) -> ChunkBitmap {
        let mut bytes = vec![0u8; count.div_ceil(8) as usize];
        for index in 0..prefix {
            bytes[(index / 8) as usize] |= 1 << (index % 8);
        }
        ChunkBitmap::new(count, bytes).expect("bitmap")
    }

    #[test]
    fn resume_starts_at_first_unverified_chunk() {
        let (local, peer) = flags();
        let count = 12;
        let verified = 7;
        let map = bitmap(verified, count);
        let decision = resume_from_checkpoint(
            local,
            peer,
            "tx-1",
            12 * STREAM_CHUNK_BYTES as u64,
            3,
            1,
            verified * STREAM_CHUNK_BYTES as u64,
            &map,
            &[0x44; 32],
        )
        .expect("resume decision");
        assert_eq!(decision, ResumeDecision::FromChunk(verified));
        assert!(map.is_verified(verified - 1));
        assert!(!map.is_verified(verified));
    }

    #[test]
    fn peer_without_capability_starts_from_zero() {
        let decision = resume_from_checkpoint(
            CAP_RESUMABLE_TRANSFER,
            0,
            "tx-1",
            12 * STREAM_CHUNK_BYTES as u64,
            3,
            1,
            0,
            &bitmap(7, 12),
            &[0x44; 32],
        )
        .expect("resume decision");
        assert_eq!(decision, ResumeDecision::Unavailable);
    }

    #[test]
    fn invalid_bitmap_is_rejected_before_resume() {
        let mut bytes = vec![0u8; 2];
        bytes[0] = 0b0000_0101;
        let err = ChunkBitmap::new(12, bytes).expect_err("gap must be rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
    }

    #[test]
    fn bitmap_bit_beyond_chunk_count_is_rejected() {
        let err = ChunkBitmap::new(3, vec![0b0000_1000]).expect_err("extra bit must be rejected");
        assert!(matches!(err, KeynearError::Validation { .. }));
    }

    #[test]
    fn stale_checkpoint_sequence_is_ignored() {
        let (local, peer) = flags();
        let decision = resume_from_checkpoint(
            local,
            peer,
            "tx-1",
            12 * STREAM_CHUNK_BYTES as u64,
            2,
            2,
            7 * STREAM_CHUNK_BYTES as u64,
            &bitmap(7, 12),
            &[0x44; 32],
        )
        .expect("resume decision");
        assert_eq!(decision, ResumeDecision::Unavailable);
    }

    #[test]
    fn verified_bytes_must_match_bitmap() {
        let (local, peer) = flags();
        let err = resume_from_checkpoint(
            local,
            peer,
            "tx-1",
            12 * STREAM_CHUNK_BYTES as u64,
            3,
            1,
            8 * STREAM_CHUNK_BYTES as u64,
            &bitmap(7, 12),
            &[0x44; 32],
        )
        .expect_err("mismatched byte count must fail");
        assert!(matches!(err, KeynearError::Validation { .. }));
    }

    #[test]
    fn rolling_chain_ends_at_final_digest() {
        let first = vec![0x11u8; STREAM_CHUNK_BYTES];
        let second = vec![0x22u8; 32];
        let mut expected = Sha256::new();
        expected.update(&first);
        expected.update(&second);
        let expected: [u8; 32] = expected.finalize().into();

        let mut chain = Sha256Chain::new();
        chain.extend(&first).expect("extend first");
        chain.extend(&second).expect("extend final");
        assert_eq!(chain.chunks(), 2);
        assert_eq!(chain.finalize(), expected);
    }

    #[test]
    fn rolling_chain_rejects_oversized_chunk() {
        let mut chain = Sha256Chain::new();
        let err = chain
            .extend(&vec![0u8; STREAM_CHUNK_BYTES + 1])
            .expect_err("oversized chunk must fail");
        assert!(matches!(err, KeynearError::Transfer(_)));
    }
}
