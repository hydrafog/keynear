use keynear_protocol::{KeynearError, STREAM_CHUNK_BYTES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPlan {
    pub offset: u64,
    pub length: usize,
    pub chunks: u64,
}

pub fn read_plan(file_size: u64, offset: u64, max_bytes: usize) -> Result<ReadPlan, KeynearError> {
    if max_bytes == 0 || max_bytes > STREAM_CHUNK_BYTES {
        return Err(KeynearError::Validation {
            field: "maxBytes",
            message: "read request must be 1..STREAM_CHUNK_BYTES bytes".to_string(),
        });
    }
    let clamped = offset.min(file_size);
    let available = (file_size - clamped) as usize;
    let length = available.min(max_bytes);
    Ok(ReadPlan {
        offset: clamped,
        length,
        chunks: length.div_ceil(STREAM_CHUNK_BYTES) as u64,
    })
}
