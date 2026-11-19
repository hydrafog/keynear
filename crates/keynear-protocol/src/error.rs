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

    #[error("cryptographic error: {0}")]
    Crypto(String),

    #[error("pairing verification timed out after 120 seconds")]
    PairingExpired,

    #[error("pairing rejected")]
    PairingRejected,

    #[error("keyring error: {0}")]
    Keyring(String),

    #[error("bluetooth error: {0}")]
    Bluetooth(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("transfer error: {0}")]
    Transfer(String),

    #[error("transfer cancelled: {0}")]
    TransferCancelled(String),

    #[error("unauthorized peer: {0}")]
    UnauthorizedPeer(String),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}
