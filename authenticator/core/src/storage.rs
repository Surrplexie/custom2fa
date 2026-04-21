use crate::account::Account;
use crate::crypto::{decrypt, encrypt};
use crate::error::AuthError;
use std::fs;
use std::path::Path;

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