use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeynearError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("validation error on field '{field}': {message}")]
    Validation {
        field: &'static str,
        message: String,
    },

    #[error("control frame size {size} bytes exceeds maximum {max} bytes")]
    FrameTooLarge { size: usize, max: usize },

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}
