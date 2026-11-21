use super::*;

pub fn build_orion_crypto_provider() -> Arc<CryptoProvider> {
    let mut provider = aws_lc_rs::default_provider();
    provider.kx_groups = vec![aws_lc_rs::kx_group::X25519MLKEM768];
    provider.cipher_suites = vec![
        aws_lc_rs::cipher_suite::TLS13_AES_256_GCM_SHA384,
        aws_lc_rs::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256,
    ];
    Arc::new(provider)
}

pub fn compute_handshake_auth_proof(
    reconnect_secret: &[u8; 32],
    local_endpoint_id: &EndpointId,
    remote_endpoint_id: &EndpointId,
    local_device_id: &str,
) -> Result<[u8; 32], KeynearError> {
    let clean_id = sanitize_device_id(local_device_id)?;
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(reconnect_secret)
        .map_err(|err| KeynearError::Crypto(format!("invalid hmac key: {err}")))?;
    mac.update(HANDSHAKE_AUTH_DOMAIN);
    mac.update(local_endpoint_id.as_bytes());
    mac.update(remote_endpoint_id.as_bytes());
    mac.update(clean_id.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn verify_handshake_auth_proof(
    reconnect_secret: &[u8; 32],
    sender_endpoint_id: &EndpointId,
    receiver_endpoint_id: &EndpointId,
    sender_device_id: &str,
    proof: &[u8],
) -> Result<(), KeynearError> {
    if proof.len() != 32 {
        return Err(KeynearError::UnauthorizedPeer(
            "invalid handshake auth proof length".to_string(),
        ));
    }
    let expected = compute_handshake_auth_proof(
        reconnect_secret,
        sender_endpoint_id,
        receiver_endpoint_id,
        sender_device_id,
    )?;
    if !bool::from(proof.ct_eq(&expected)) {
        return Err(KeynearError::UnauthorizedPeer(
            "handshake auth proof verification failed".to_string(),
        ));
    }
    Ok(())
}
