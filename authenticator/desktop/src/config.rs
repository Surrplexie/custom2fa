use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const APP_DIR: &str = "custom2fa";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub db_path: String,
    #[serde(default)]
    pub hide_codes: bool,
    /// Idle seconds before the vault locks. `0` disables auto-lock.
    #[serde(default = "default_auto_lock")]
    pub auto_lock_seconds: u32,
    #[serde(default = "default_width")]
    pub window_width: f32,
    #[serde(default = "default_height")]
    pub window_height: f32,
    /// If true, load the keychain passphrase and unlock the vault on launch.
    #[serde(default = "default_true")]
    pub auto_unlock: bool,
}

fn default_auto_lock() -> u32 {
    300
}
fn default_width() -> f32 {
    980.0
}
fn default_height() -> f32 {
    700.0
}
fn default_true() -> bool {
    true
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            db_path: default_vault_path(),
            hide_codes: false,
            auto_lock_seconds: default_auto_lock(),
            window_width: default_width(),
            window_height: default_height(),
            auto_unlock: true,
        }
    }
}

pub fn default_vault_path() -> String {
    data_dir()
        .join("accounts.c2fa")
        .to_string_lossy()
        .into_owned()
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

pub fn load() -> UiConfig {
    let path = config_path();
    let mut cfg = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<UiConfig>(&bytes).unwrap_or_default(),
        Err(_) => UiConfig::default(),
    };
    cfg.db_path = resolve_vault_path(&cfg.db_path);
    cfg
}

/// Prefer an existing vault over a brand-new default path.
/// The pre-0.3 GUI stored a relative `!2fa` file (often next to the exe).
pub fn resolve_vault_path(configured: &str) -> String {
    let configured = configured.trim();
    if !configured.is_empty() && Path::new(configured).is_file() {
        return configured.to_string();
    }

    for candidate in existing_vault_candidates() {
        if candidate.is_file() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    if !configured.is_empty() {
        return configured.to_string();
    }
    default_vault_path()
}

fn existing_vault_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let names = ["!2fa", "accounts.c2fa"];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in names {
                out.push(dir.join(name));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for name in names {
            out.push(cwd.join(name));
        }
    }
    out.push(PathBuf::from(r"E:\!apps\!2fa"));
    out.push(PathBuf::from(default_vault_path()));
    out
}

pub fn save(cfg: &UiConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(cfg) {
        let _ = fs::write(path, bytes);
    }
}
