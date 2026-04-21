use crate::account::Account;
use crate::crypto::{decrypt, encrypt};
use crate::error::AuthError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use time::OffsetDateTime;
use zeroize::Zeroize;

pub fn save_accounts(path: &Path, accounts: &[Account], passphrase: &str) -> Result<(), AuthError> {
    let data = serde_json::to_vec(accounts)?;
    let encrypted = encrypt(&data, passphrase)?;
    fs::write(path, encrypted)?;
    Ok(())
}

pub fn load_accounts(path: &Path, passphrase: &str) -> Result<Vec<Account>, AuthError> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let encrypted = fs::read(path)?;
    let decrypted = decrypt(&encrypted, passphrase)?;
    let accounts: Vec<Account> = serde_json::from_slice(&decrypted)?;
    Ok(accounts)
}

#[derive(Serialize, Deserialize)]
struct BackupFile {
    format: String,
    version: u8,
    created_at_unix: i64,
    payload_b64: String,
}

pub fn export_backup(
    db_path: &Path,
    backup_path: &Path,
    db_passphrase: &str,
    backup_passphrase: &str,
) -> Result<(), AuthError> {
    let accounts = load_accounts(db_path, db_passphrase)?;
    let mut plaintext = serde_json::to_vec(&accounts)?;
    let backup_ciphertext = encrypt(&plaintext, backup_passphrase)?;
    plaintext.zeroize();

    let backup = BackupFile {
        format: "custom2fa-backup".to_string(),
        version: 1,
        created_at_unix: OffsetDateTime::now_utc().unix_timestamp(),
        payload_b64: STANDARD.encode(backup_ciphertext),
    };

    let serialized = serde_json::to_vec_pretty(&backup)?;
    fs::write(backup_path, serialized)?;
    Ok(())
}

pub fn import_backup(
    backup_path: &Path,
    db_path: &Path,
    backup_passphrase: &str,
    db_passphrase: &str,
) -> Result<(), AuthError> {
    let serialized = fs::read(backup_path)?;
    let backup: BackupFile = serde_json::from_slice(&serialized)?;
    if backup.format != "custom2fa-backup" || backup.version != 1 {
        return Err(AuthError::InvalidCiphertext);
    }

    let mut payload = STANDARD.decode(backup.payload_b64)?;
    let mut plaintext = decrypt(&payload, backup_passphrase)?;
    payload.zeroize();
    let accounts: Vec<Account> = serde_json::from_slice(&plaintext)?;
    plaintext.zeroize();
    save_accounts(db_path, &accounts, db_passphrase)?;
    Ok(())
}