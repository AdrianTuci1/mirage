use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verify an Ed25519 signature over raw bytes.
///
/// The `signature` argument is expected to be a base64-encoded Ed25519 signature.
/// The `public_key` is a 32-byte Ed25519 public key.
pub fn verify_ed25519(bytes: &[u8], signature_b64: &str, public_key: &[u8; 32]) -> Result<()> {
    let vk = VerifyingKey::from_bytes(public_key)
        .map_err(|e| anyhow!("invalid ed25519 public key: {}", e))?;
    let sig_bytes = STANDARD
        .decode(signature_b64)
        .context("failed to base64-decode signature")?;
    let signature_bytes: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|v: Vec<u8>| anyhow!("expected 64-byte ed25519 signature, got {}", v.len()))?;
    let signature = Signature::from_bytes(&signature_bytes);
    vk.verify(bytes, &signature)
        .context("catalog signature verification failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    #[test]
    fn verify_known_good_signature() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let message = b"catalog content";
        let signature = signing_key.sign(message);
        let signature_b64 = STANDARD.encode(signature.to_bytes());

        assert!(verify_ed25519(message, &signature_b64, verifying_key.as_bytes()).is_ok());
    }

    #[test]
    fn reject_bad_signature() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let message = b"catalog content";
        let signature_b64 = STANDARD.encode([0u8; 64]);

        assert!(verify_ed25519(message, &signature_b64, verifying_key.as_bytes()).is_err());
    }
}
