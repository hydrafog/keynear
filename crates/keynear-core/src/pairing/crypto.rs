use std::fmt;

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use keynear_protocol::KeynearError;

pub const PAIRING_PROTOCOL: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const MAX_GATT_PAIRING_FRAME_BYTES: usize = 512;
pub const EXPORT_CONTRIBUTION_LEN: usize = 32;
pub(crate) const MAX_DISPLAY_NAME_BYTES: usize = 64;
const RECONNECT_KEY_INFO: &[u8] = b"keynear-cygnus-reconnect-v1";
pub(crate) const METADATA_FORMAT_VERSION: u8 = 0x01;

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct SecretBytes32(pub(crate) [u8; 32]);

impl fmt::Debug for SecretBytes32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

pub(crate) fn verification_code_from_hash(hash_arr: &[u8; 32]) -> String {
    let code_u32 = u32::from_be_bytes([hash_arr[0], hash_arr[1], hash_arr[2], hash_arr[3]]);
    format!("{:06}", code_u32 % 1_000_000)
}

pub(crate) fn derive_reconnect_secret(
    handshake_hash: &[u8; 32],
    ikm: &[u8; 64],
) -> Result<[u8; 32], KeynearError> {
    let hk = Hkdf::<Sha256>::new(Some(handshake_hash), &ikm[..]);
    let mut reconnect_secret = [0u8; 32];
    hk.expand(RECONNECT_KEY_INFO, &mut reconnect_secret)
        .map_err(|_| KeynearError::Crypto("hkdf expand failed".to_string()))?;
    Ok(reconnect_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_code_from_hash_is_six_digits_and_wraps_modulo() {
        let mut zeroed = [0u8; 32];
        assert_eq!(verification_code_from_hash(&zeroed), "000000");
        zeroed[3] = 5;
        assert_eq!(verification_code_from_hash(&zeroed), "000005");
        assert_eq!(verification_code_from_hash(&[0xFF; 32]), "967295");
    }

    #[test]
    fn derive_reconnect_secret_is_deterministic_and_domain_separated() {
        let handshake_hash = [0x11; 32];
        let ikm = [0x22; 64];
        let first = derive_reconnect_secret(&handshake_hash, &ikm).expect("first secret");
        assert_eq!(
            first,
            derive_reconnect_secret(&handshake_hash, &ikm).expect("repeat secret")
        );

        let mut other_hash = handshake_hash;
        other_hash[0] = 0x33;
        assert_ne!(
            first,
            derive_reconnect_secret(&other_hash, &ikm).expect("other hash")
        );
        let mut other_ikm = ikm;
        other_ikm[0] = 0x44;
        assert_ne!(
            first,
            derive_reconnect_secret(&handshake_hash, &other_ikm).expect("other ikm")
        );
        assert_eq!(first.len(), 32);
    }
}
