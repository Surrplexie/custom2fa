use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    let Ok(bytes) = fs::read(&path) else {
        return UiConfig::default();
    };
    match serde_json::from_slice::<UiConfig>(&bytes) {
        Ok(mut cfg) => {
            if cfg.db_path.trim().is_empty() || cfg.db_path.trim() == "!2fa" {
                cfg.db_path = default_vault_path();
            }
            cfg
        }
        Err(_) => UiConfig::default(),
    }
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
