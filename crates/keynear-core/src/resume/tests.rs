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
    let digest = [0x44; 32];
    let input = CheckpointInput {
        local_flags: local,
        peer_flags: peer,
        transfer_id: "tx-1",
        total_bytes: 12 * STREAM_CHUNK_BYTES as u64,
        sequence: 3,
        last_sequence: 1,
        verified_bytes: verified * STREAM_CHUNK_BYTES as u64,
        bitmap: &map,
        rolling_digest: &digest,
    };
    let decision = resume_from_checkpoint(input).expect("resume decision");
    assert_eq!(decision, ResumeDecision::FromChunk(verified));
    assert!(map.is_verified(verified - 1));
    assert!(!map.is_verified(verified));
}

#[test]
fn peer_without_capability_starts_from_zero() {
    let map = bitmap(7, 12);
    let digest = [0x44; 32];
    let input = CheckpointInput {
        local_flags: CAP_RESUMABLE_TRANSFER,
        peer_flags: 0,
        transfer_id: "tx-1",
        total_bytes: 12 * STREAM_CHUNK_BYTES as u64,
        sequence: 3,
        last_sequence: 1,
        verified_bytes: 0,
        bitmap: &map,
        rolling_digest: &digest,
    };
    let decision = resume_from_checkpoint(input).expect("resume decision");
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
    let map = bitmap(7, 12);
    let digest = [0x44; 32];
    let input = CheckpointInput {
        local_flags: local,
        peer_flags: peer,
        transfer_id: "tx-1",
        total_bytes: 12 * STREAM_CHUNK_BYTES as u64,
        sequence: 2,
        last_sequence: 2,
        verified_bytes: 7 * STREAM_CHUNK_BYTES as u64,
        bitmap: &map,
        rolling_digest: &digest,
    };
    let decision = resume_from_checkpoint(input).expect("resume decision");
    assert_eq!(decision, ResumeDecision::Unavailable);
}

#[test]
fn verified_bytes_must_match_bitmap() {
    let (local, peer) = flags();
    let map = bitmap(7, 12);
    let digest = [0x44; 32];
    let input = CheckpointInput {
        local_flags: local,
        peer_flags: peer,
        transfer_id: "tx-1",
        total_bytes: 12 * STREAM_CHUNK_BYTES as u64,
        sequence: 3,
        last_sequence: 1,
        verified_bytes: 8 * STREAM_CHUNK_BYTES as u64,
        bitmap: &map,
        rolling_digest: &digest,
    };
    let err = resume_from_checkpoint(input).expect_err("mismatched byte count must fail");
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
