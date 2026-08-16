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

    let stem = path.file_name().and_then(|n| n.to_str()).unwrap_or("vault");
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
    use std::os::windows::ffi::OsStrExt;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    extern "system" {
        fn MoveFileExW(
            lp_existing_file_name: *const u16,
            lp_new_file_name: *const u16,
            dw_flags: u32,
        ) -> i32;
    }

    let from_w = wide(from);
    let to_w = wide(to);
    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        let err = std::io::Error::last_os_error();
        let _ = fs::remove_file(from);
        return Err(AuthError::Io(err));
    }
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

/// Re-encrypts an existing vault with a new passphrase.
pub fn change_passphrase(
    path: &Path,
    old_passphrase: &str,
    new_passphrase: &str,
) -> Result<(), AuthError> {
    if new_passphrase.is_empty() {
        return Err(AuthError::InvalidPassphrase);
    }
    let accounts = load_accounts(path, old_passphrase)?;
    save_accounts(path, &accounts, new_passphrase)
}

/// Result of importing a backup into a vault.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportBackupResult {
    pub added: usize,
    pub skipped_labels: Vec<String>,
    /// `(incoming_label, existing_label)` when secrets match but labels differ.
    pub duplicate_secrets: Vec<(String, String)>,
    pub replaced: bool,
}

/// Merges `incoming` into `existing`. Accounts with a duplicate label are skipped.
pub fn merge_accounts(existing: &mut Vec<Account>, incoming: Vec<Account>) -> ImportBackupResult {
    let mut result = ImportBackupResult::default();
    for account in incoming {
        if existing.iter().any(|e| e.label == account.label) {
            result.skipped_labels.push(account.label);
            continue;
        }
        if let Some(other) = existing.iter().find(|e| e.secret == account.secret) {
            result
                .duplicate_secrets
                .push((account.label.clone(), other.label.clone()));
        }
        existing.push(account);
        result.added += 1;
    }
    result
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
    replace: bool,
) -> Result<ImportBackupResult, AuthError> {
    let serialized = fs::read(backup_path)?;
    let backup: BackupFile = serde_json::from_slice(&serialized)?;
    if backup.format != "custom2fa-backup" || backup.version != 1 {
        return Err(AuthError::InvalidCiphertext);
    }

    let mut payload = STANDARD.decode(backup.payload_b64)?;
    let mut plaintext = decrypt(&payload, backup_passphrase)?;
    payload.zeroize();
    let incoming: Vec<Account> = serde_json::from_slice(&plaintext)?;
    plaintext.zeroize();

    if replace {
        let added = incoming.len();
        save_accounts(db_path, &incoming, db_passphrase)?;
        return Ok(ImportBackupResult {
            added,
            skipped_labels: Vec::new(),
            duplicate_secrets: Vec::new(),
            replaced: true,
        });
    }

    let mut existing = load_accounts(db_path, db_passphrase)?;
    let result = merge_accounts(&mut existing, incoming);
    save_accounts(db_path, &existing, db_passphrase)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{
        change_passphrase, export_backup, import_backup, load_accounts, merge_accounts,
        save_accounts, write_file_atomic,
    };
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
        assert!(
            !temp_left,
            "temp file should be removed after successful rename"
        );

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

    #[test]
    fn change_passphrase_roundtrip() {
        let path = test_vault_path("changepass");
        let _ = fs::remove_file(&path);
        save_accounts(&path, &[sample_account()], "old-pass").unwrap();
        change_passphrase(&path, "old-pass", "new-pass").unwrap();
        assert!(load_accounts(&path, "old-pass").is_err());
        let loaded = load_accounts(&path, "new-pass").unwrap();
        assert_eq!(loaded[0].label, "user@example.com");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn merge_skips_duplicate_labels_and_notes_duplicate_secrets() {
        let mut existing = vec![sample_account()];
        let same_label = sample_account();
        let mut same_secret = sample_account();
        same_secret.label = "other@example.com".into();
        let mut fresh = sample_account();
        fresh.label = "fresh@example.com".into();
        fresh.secret = b"different-secret-bytes".to_vec();

        let result = merge_accounts(&mut existing, vec![same_label, same_secret, fresh]);
        assert_eq!(result.added, 2);
        assert_eq!(result.skipped_labels, vec!["user@example.com"]);
        assert_eq!(
            result.duplicate_secrets,
            vec![("other@example.com".into(), "user@example.com".into())]
        );
        assert_eq!(existing.len(), 3);
    }

    #[test]
    fn import_backup_merges_by_default() {
        let db = test_vault_path("merge_db");
        let bak = test_vault_path("merge_bak.json");
        let _ = fs::remove_file(&db);
        let _ = fs::remove_file(&bak);

        save_accounts(&db, &[sample_account()], "db-pass").unwrap();

        let mut extra = sample_account();
        extra.label = "second@example.com".into();
        extra.secret = b"another-secret".to_vec();
        save_accounts(&test_vault_path("merge_src"), &[extra], "db-pass").unwrap();
        // Use a dedicated source vault so export_backup reads from it.
        let src = test_vault_path("merge_src");
        export_backup(&src, &bak, "db-pass", "bak-pass").unwrap();

        let result = import_backup(&bak, &db, "bak-pass", "db-pass", false).unwrap();
        assert_eq!(result.added, 1);
        assert!(result.skipped_labels.is_empty());
        let loaded = load_accounts(&db, "db-pass").unwrap();
        assert_eq!(loaded.len(), 2);

        let _ = fs::remove_file(&db);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_file(&src);
    }

    #[test]
    fn import_backup_replace_overwrites_vault() {
        let db = test_vault_path("repl_db");
        let bak = test_vault_path("repl_bak.json");
        let src = test_vault_path("repl_src");
        let _ = fs::remove_file(&db);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_file(&src);

        save_accounts(&db, &[sample_account()], "db-pass").unwrap();
        let mut extra = sample_account();
        extra.label = "only-in-backup".into();
        extra.secret = b"backup-only".to_vec();
        save_accounts(&src, &[extra], "db-pass").unwrap();
        export_backup(&src, &bak, "db-pass", "bak-pass").unwrap();

        let result = import_backup(&bak, &db, "bak-pass", "db-pass", true).unwrap();
        assert!(result.replaced);
        assert_eq!(result.added, 1);
        let loaded = load_accounts(&db, "db-pass").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "only-in-backup");

        let _ = fs::remove_file(&db);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_file(&src);
    }
}
