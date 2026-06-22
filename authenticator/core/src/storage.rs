use crate::account::Account;
use crate::crypto::{decrypt, encrypt};
use crate::error::AuthError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use time::OffsetDateTime;
use zeroize::Zeroize;

/// Writes `data` to `path` atomically: temp file in the same directory, fsync, then rename.
fn write_file_atomic(path: &Path, data: &[u8]) -> Result<(), AuthError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    fs::create_dir_all(parent)?;

    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("vault");
    let temp_path = parent.join(format!(".{stem}.{}.tmp", std::process::id()));

    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    atomic_replace(&temp_path, path)?;
    Ok(())
}

#[cfg(unix)]
fn atomic_replace(from: &Path, to: &Path) -> Result<(), AuthError> {
    fs::rename(from, to).map_err(|e| {
        let _ = fs::remove_file(from);
        AuthError::Io(e)
    })?;
    Ok(())
}

#[cfg(windows)]
fn atomic_replace(from: &Path, to: &Path) -> Result<(), AuthError> {
    if to.exists() {
        fs::remove_file(to)?;
    }
    fs::rename(from, to).map_err(|e| {
        let _ = fs::remove_file(from);
        AuthError::Io(e)
    })?;
    Ok(())
}

pub fn save_accounts(path: &Path, accounts: &[Account], passphrase: &str) -> Result<(), AuthError> {
    let data = serde_json::to_vec(accounts)?;
    let encrypted = encrypt(&data, passphrase)?;
    write_file_atomic(path, &encrypted)?;
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
    write_file_atomic(backup_path, &serialized)?;
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

#[cfg(test)]
mod tests {
    use super::{load_accounts, save_accounts, write_file_atomic};
    use crate::account::{Account, TotpAlgorithm};
    use std::fs;
    use std::path::PathBuf;

    fn test_vault_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "custom2fa_storage_test_{}_{name}",
            std::process::id()
        ))
    }

    fn sample_account() -> Account {
        Account {
            issuer: "Test".into(),
            label: "user@example.com".into(),
            secret: b"super-secret-key-material".to_vec(),
            algorithm: TotpAlgorithm::Sha1,
            period_seconds: 30,
            digits: 6,
            category: String::new(),
        }
    }

    #[test]
    fn save_load_roundtrip_via_atomic_write() {
        let path = test_vault_path("roundtrip");
        let _ = fs::remove_file(&path);

        let accounts = vec![sample_account()];
        save_accounts(&path, &accounts, "test-passphrase").expect("save should succeed");

        let loaded = load_accounts(&path, "test-passphrase").expect("load should succeed");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "user@example.com");
        assert_eq!(loaded[0].secret, b"super-secret-key-material");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn atomic_write_leaves_no_temp_files() {
        let path = test_vault_path("no_temp");
        let parent = path.parent().unwrap();
        let _ = fs::remove_file(&path);

        write_file_atomic(&path, b"payload").expect("atomic write should succeed");
        assert!(path.exists());

        let temp_left = fs::read_dir(parent)
            .expect("read_dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.contains(".no_temp.") && name.ends_with(".tmp"))
            });
        assert!(!temp_left, "temp file should be removed after successful rename");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let path = test_vault_path("replace");
        let _ = fs::remove_file(&path);

        write_file_atomic(&path, b"first").expect("first write");
        write_file_atomic(&path, b"second").expect("replace write");

        let contents = fs::read(&path).expect("read vault");
        assert_eq!(contents, b"second");

        let _ = fs::remove_file(&path);
    }
}
