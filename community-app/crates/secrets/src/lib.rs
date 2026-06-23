//! Application-level AES-256-GCM encryption for user-supplied secrets (LLM API keys).
//!
//! There is no encryption-at-rest elsewhere in this codebase (refresh tokens are SHA-256
//! hashed, passwords Argon2 hashed) because nothing else needs to be *reversible*. BYO LLM
//! credentials do, since the bridge needs the plaintext key to call the provider — so this
//! crate exists specifically for that one case. The master key never leaves this process;
//! decryption happens in the API server right before an outbound provider call.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;

const KEY_ENV: &str = "LORELEI_CREDENTIALS_KEY";
const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("{0} is not set")]
    MissingKey(&'static str),
    #[error("{0} is not valid base64")]
    InvalidKeyEncoding(&'static str),
    #[error("{0} must decode to exactly 32 bytes (got {1})")]
    InvalidKeyLength(&'static str, usize),
    #[error("ciphertext is shorter than the nonce")]
    CiphertextTooShort,
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed (wrong key or corrupted data)")]
    DecryptionFailed,
}

/// The master key used to encrypt/decrypt stored LLM credentials, loaded once at boot.
pub struct CredentialsKey([u8; 32]);

impl CredentialsKey {
    /// Loads and validates `LORELEI_CREDENTIALS_KEY` (32 raw bytes, base64-encoded).
    /// Call this once at startup so a missing/malformed key fails fast instead of at the
    /// first credential save.
    pub fn from_env() -> Result<Self, SecretsError> {
        let raw = std::env::var(KEY_ENV).map_err(|_| SecretsError::MissingKey(KEY_ENV))?;
        Self::from_base64(&raw)
    }

    pub fn from_base64(raw: &str) -> Result<Self, SecretsError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|_| SecretsError::InvalidKeyEncoding(KEY_ENV))?;
        let len = bytes.len();
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| SecretsError::InvalidKeyLength(KEY_ENV, len))?;
        Ok(Self(arr))
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.0))
    }
}

/// Encrypts `plaintext`, returning `nonce || ciphertext`. The nonce is random per call and
/// safe to store alongside the ciphertext (it is not secret).
pub fn encrypt(plaintext: &str, key: &CredentialsKey) -> Result<Vec<u8>, SecretsError> {
    let cipher = key.cipher();
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| SecretsError::EncryptionFailed)?;
    let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts a blob produced by [`encrypt`].
pub fn decrypt(blob: &[u8], key: &CredentialsKey) -> Result<String, SecretsError> {
    if blob.len() < NONCE_LEN {
        return Err(SecretsError::CiphertextTooShort);
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = key
        .cipher()
        .decrypt(nonce, ciphertext)
        .map_err(|_| SecretsError::DecryptionFailed)?;
    String::from_utf8(plaintext).map_err(|_| SecretsError::DecryptionFailed)
}

/// A display-safe fingerprint (e.g. `"sk-p...AbCd"`) for showing a configured credential in
/// the UI without ever decrypting it. Computed from the plaintext once, at save time, and
/// stored alongside the ciphertext — never recomputed from a decrypted value on read.
pub fn fingerprint(plaintext: &str) -> String {
    let chars: Vec<char> = plaintext.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(4));
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}...{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> CredentialsKey {
        CredentialsKey([7u8; 32])
    }

    #[test]
    fn round_trips_plaintext() {
        let key = test_key();
        let blob = encrypt("sk-super-secret-key", &key).unwrap();
        assert_eq!(decrypt(&blob, &key).unwrap(), "sk-super-secret-key");
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let blob = encrypt("sk-super-secret-key", &test_key()).unwrap();
        let other = CredentialsKey([9u8; 32]);
        assert!(decrypt(&blob, &other).is_err());
    }

    #[test]
    fn two_encryptions_of_same_plaintext_differ() {
        let key = test_key();
        let a = encrypt("same-value", &key).unwrap();
        let b = encrypt("same-value", &key).unwrap();
        assert_ne!(a, b, "nonce should make ciphertexts differ");
    }

    #[test]
    fn fingerprint_masks_middle() {
        assert_eq!(fingerprint("sk-1234567890abcd"), "sk-1...abcd");
        assert_eq!(fingerprint("short"), "*****");
    }

    #[test]
    fn rejects_malformed_key() {
        assert!(CredentialsKey::from_base64("not-base64!!").is_err());
        assert!(CredentialsKey::from_base64("YWJj").is_err()); // valid b64, wrong length
    }
}
