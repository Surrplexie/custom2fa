use crate::error::AuthError;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 210_000;
const MAGIC: &[u8; 4] = b"C2FA";
const VERSION: u8 = 1;

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn encrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, AuthError> {
    if passphrase.is_empty() {
        return Err(AuthError::InvalidPassphrase);
    }

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AuthError::EncryptionError)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|_| AuthError::EncryptionError)?;

    let mut out = Vec::with_capacity(4 + 1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, AuthError> {
    if passphrase.is_empty() {
        return Err(AuthError::InvalidPassphrase);
    }
    if data.len() < 4 + 1 + SALT_LEN + NONCE_LEN {
        return Err(AuthError::InvalidCiphertext);
    }
    if &data[..4] != MAGIC {
        return Err(AuthError::InvalidCiphertext);
    }
    if data[4] != VERSION {
        return Err(AuthError::UnsupportedVersion);
    }

    let salt_start = 5;
    let nonce_start = salt_start + SALT_LEN;
    let ct_start = nonce_start + NONCE_LEN;
    let salt = &data[salt_start..nonce_start];
    let nonce_bytes = &data[nonce_start..ct_start];
    let ciphertext = &data[ct_start..];

    let key = derive_key(passphrase, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AuthError::EncryptionError)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AuthError::DecryptionError)
}