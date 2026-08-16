// Custom2FA Desktop Hub — modernised GUI
// Dark theme · sidebar · live-refresh codes · countdown timer · categories
// Offline-first TOTP — no network required for normal use.

// Suppress the Windows console window when launched as a GUI app.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod config;

use custom2fa_core::account::{
    label_for_secret, validate_digits, validate_period, zeroize_accounts, Account, TotpAlgorithm,
};
use custom2fa_core::otp_uri::{
    parse_otpauth_uri, parse_otpauth_uri_from_luma, parse_otpauth_uri_from_qr_image,
};
use custom2fa_core::storage::{
    change_passphrase, export_backup, import_backup, load_accounts, save_accounts,
};
use custom2fa_core::totp::{decode_secret, format_totp_code, generate_totp_for_account};
use eframe::egui::{self, Color32, Key, Modifiers, RichText, Stroke};
use keyring::Entry;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

// ── Theme constants ──────────────────────────────────────────────────────────
const C_PANEL: Color32 = Color32::from_rgb(22, 22, 29);
const C_CARD: Color32 = Color32::from_rgb(31, 35, 53);
const C_BORDER: Color32 = Color32::from_rgb(61, 89, 161);
const C_ACCENT: Color32 = Color32::from_rgb(122, 162, 247);
const C_TEXT: Color32 = Color32::from_rgb(192, 202, 245);
const C_MUTED: Color32 = Color32::from_rgb(86, 95, 137);
const C_OK: Color32 = Color32::from_rgb(158, 206, 106);
const C_ERR: Color32 = Color32::from_rgb(247, 118, 142);
const C_WARN: Color32 = Color32::from_rgb(224, 175, 104);

fn build_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = C_PANEL;
    v.window_fill = C_CARD;
    v.window_stroke = Stroke::new(1.0, C_BORDER);
    v.widgets.noninteractive.bg_fill = C_CARD;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, C_TEXT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, C_BORDER);
    v.widgets.inactive.bg_fill = Color32::from_rgb(44, 50, 74);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, C_TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(55, 63, 90);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, C_ACCENT);
    v.widgets.active.bg_fill = Color32::from_rgb(61, 89, 161);
    v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    v.selection.bg_fill = Color32::from_rgb(40, 60, 110);
    v.selection.stroke = Stroke::new(1.0, C_ACCENT);
    v.extreme_bg_color = Color32::from_rgb(16, 16, 24);
    v.faint_bg_color = Color32::from_rgb(26, 30, 46);
    v
}

// ── Navigation ───────────────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Copy, Default, Debug)]
enum Panel {
    #[default]
    Accounts,
    Add,
    Backup,
    Settings,
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
enum AddTab {
    #[default]
    Manual,
    Uri,
    QrImage,
    Camera,
}

// ── Card display data (collected before rendering to avoid borrow conflicts) ─
struct CardData {
    issuer: String,
    label: String,
    category: String,
    algo: String,
    digits: u8,
    period: u32,
    code: String,
    raw_code: String,
    secs: u32,
    frac: f32,
    expanded: bool,
    hidden: bool,
}

enum CardAction {
    Edit(String),
    Delete(String),
    Copied { label: String, code: String },
    Toggle(String),
}

const CLIPBOARD_TTL: Duration = Duration::from_secs(30);

// ── App state ────────────────────────────────────────────────────────────────
struct Custom2faApp {
    cfg: config::UiConfig,

    // vault
    db_path: String,
    db_pass: String,
    accounts_loaded: bool,
    accounts: Vec<Account>,
    live_codes: HashMap<String, (String, String, u32, f32)>, // label -> (disp, raw, secs, frac)

    // navigation / filter
    panel: Panel,
    add_tab: AddTab,
    sel_cat: String, // "" = All, "__none__" = uncategorised
    sel_label: String,
    search: String,
    search_focus: bool,

    // add-account form
    af_issuer: String,
    af_label: String,
    af_secret: String,
    af_algo: TotpAlgorithm,
    af_period: u32,
    af_digits: u8,
    af_cat: String,

    // import sub-fields
    if_uri: String,
    if_qr: String,
    if_cam: String,

    // edit window
    editing: Option<String>, // original label
    ef_issuer: String,
    ef_label: String,
    ef_secret: String,
    ef_algo: TotpAlgorithm,
    ef_period: u32,
    ef_digits: u8,
    ef_cat: String,

    // backup
    bk_path: String,
    bk_pass: String,
    backup_prompt: bool,

    // change passphrase
    new_pass: String,
    new_pass_confirm: String,

    // status bar
    status: String,
    is_err: bool,

    // confirm-delete
    del_label: Option<String>,

    // duplicate-secret confirm
    dup_pending: Option<Account>,

    // which account cards are currently expanded / revealed
    expanded_labels: std::collections::HashSet<String>,
    revealed_labels: std::collections::HashSet<String>,

    last_input: Instant,
    clipboard_clear_at: Option<Instant>,
    clipboard_value: String,
}

impl Default for Custom2faApp {
    fn default() -> Self {
        Self::from_config(config::UiConfig::default())
    }
}

impl Custom2faApp {
    fn from_config(cfg: config::UiConfig) -> Self {
        let db_path = cfg.db_path.clone();
        Self {
            cfg,
            db_path,
            db_pass: String::new(),
            accounts_loaded: false,
            accounts: Vec::new(),
            live_codes: HashMap::new(),

            panel: Panel::default(),
            add_tab: AddTab::default(),
            sel_cat: String::new(),
            sel_label: String::new(),
            search: String::new(),
            search_focus: false,

            af_issuer: String::new(),
            af_label: String::new(),
            af_secret: String::new(),
            af_algo: TotpAlgorithm::default(),
            af_period: 30,
            af_digits: 6,
            af_cat: String::new(),

            if_uri: String::new(),
            if_qr: String::new(),
            if_cam: "0".into(),

            editing: None,
            ef_issuer: String::new(),
            ef_label: String::new(),
            ef_secret: String::new(),
            ef_algo: TotpAlgorithm::default(),
            ef_period: 30,
            ef_digits: 6,
            ef_cat: String::new(),

            bk_path: "backup-2fa.json".into(),
            bk_pass: String::new(),
            backup_prompt: false,

            new_pass: String::new(),
            new_pass_confirm: String::new(),

            status: String::new(),
            is_err: false,

            del_label: None,
            dup_pending: None,

            expanded_labels: std::collections::HashSet::new(),
            revealed_labels: std::collections::HashSet::new(),

            last_input: Instant::now(),
            clipboard_clear_at: None,
            clipboard_value: String::new(),
        }
    }

    fn persist_cfg(&mut self) {
        self.cfg.db_path = self.db_path.clone();
        config::save(&self.cfg);
    }

    fn try_auto_unlock(&mut self) {
        if !self.cfg.auto_unlock {
            return;
        }
        if self.do_load_keychain().is_ok() && !self.db_pass.is_empty() && self.do_reload().is_ok() {
            self.set_ok("Vault unlocked from keychain.");
            self.panel = Panel::Accounts;
        }
    }

    fn pick_file(filters: &[(&str, &[&str])]) -> Option<String> {
        let mut dlg = rfd::FileDialog::new();
        for (name, exts) in filters {
            dlg = dlg.add_filter(*name, exts);
        }
        dlg.pick_file().map(|p| p.to_string_lossy().into_owned())
    }

    fn save_file(filters: &[(&str, &[&str])], filename: &str) -> Option<String> {
        let mut dlg = rfd::FileDialog::new().set_file_name(filename);
        for (name, exts) in filters {
            dlg = dlg.add_filter(*name, exts);
        }
        dlg.save_file().map(|p| p.to_string_lossy().into_owned())
    }

    fn copy_code(&mut self, ctx: &egui::Context, label: &str, code: String) {
        ctx.copy_text(code.clone());
        self.clipboard_value = code;
        self.clipboard_clear_at = Some(Instant::now() + CLIPBOARD_TTL);
        self.revealed_labels.insert(label.to_string());
        self.set_ok(format!(
            "Code for \"{label}\" copied. Clipboard clears in 30s."
        ));
    }

    fn maybe_clear_clipboard(&mut self) {
        let Some(at) = self.clipboard_clear_at else {
            return;
        };
        if Instant::now() < at {
            return;
        }
        self.clipboard_clear_at = None;
        if self.clipboard_value.is_empty() {
            return;
        }
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if cb.get_text().ok().as_deref() == Some(self.clipboard_value.as_str()) {
                let _ = cb.set_text(String::new());
                self.set_ok("Clipboard cleared.");
            }
        }
        self.clipboard_value.zeroize();
    }

    fn note_input(&mut self, ctx: &egui::Context) {
        let interacted = ctx.input(|i| {
            i.pointer.any_pressed()
                || !i.keys_down.is_empty()
                || i.raw_scroll_delta != egui::Vec2::ZERO
        });
        if interacted {
            self.last_input = Instant::now();
        }
    }

    fn maybe_auto_lock(&mut self) {
        if !self.accounts_loaded {
            return;
        }
        let secs = self.cfg.auto_lock_seconds;
        if secs == 0 {
            return;
        }
        if self.last_input.elapsed() >= Duration::from_secs(u64::from(secs)) {
            self.lock_vault();
            self.set_ok("Vault locked after idle timeout.");
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut lock = false;
        let mut add = false;
        let mut focus_search = false;
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::L) {
                lock = true;
            }
            if i.consume_key(Modifiers::COMMAND, Key::N) {
                add = true;
            }
            if i.consume_key(Modifiers::COMMAND, Key::F) {
                focus_search = true;
            }
            if (self.editing.is_some()
                || self.del_label.is_some()
                || self.backup_prompt
                || self.dup_pending.is_some())
                && i.consume_key(Modifiers::NONE, Key::Escape)
            {
                if self.editing.is_some() {
                    self.editing = None;
                } else if self.del_label.is_some() {
                    self.del_label = None;
                } else if self.backup_prompt {
                    self.backup_prompt = false;
                } else if self.dup_pending.is_some() {
                    self.dup_pending = None;
                }
            }
        });
        if lock && self.accounts_loaded {
            self.lock_vault();
            self.set_ok("Vault locked.");
        }
        if add {
            self.panel = Panel::Add;
        }
        if focus_search {
            self.search_focus = true;
        }
    }
}

impl Custom2faApp {
    // ── Helpers ──────────────────────────────────────────────────────────────
    fn db_pb(&self) -> PathBuf {
        PathBuf::from(&self.db_path)
    }

    fn clean_path(s: &str) -> String {
        s.trim().trim_matches('"').trim().to_string()
    }

    fn set_ok(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.is_err = false;
    }

    fn set_err(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.is_err = true;
    }

    fn exec<F: FnOnce(&mut Self) -> Result<(), String>>(&mut self, ok: &str, f: F) {
        match f(self) {
            Ok(_) => self.set_ok(ok),
            Err(e) => self.set_err(e),
        }
    }

    // ── Live codes ───────────────────────────────────────────────────────────
    fn refresh_live_codes(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.live_codes.clear();
        for acc in &self.accounts {
            let period = acc.period_seconds as i64;
            let elapsed = now.rem_euclid(period);
            let secs = (period - elapsed) as u32;
            let frac = elapsed as f32 / period as f32;
            let raw = match generate_totp_for_account(acc) {
                Ok(c) => format_totp_code(c, acc.digits),
                Err(_) => "------".to_string(),
            };
            let disp = match raw.len() {
                6 => format!("{} {}", &raw[..3], &raw[3..]),
                8 => format!("{} {}", &raw[..4], &raw[4..]),
                _ => raw.clone(),
            };
            self.live_codes
                .insert(acc.label.clone(), (disp, raw, secs, frac));
        }
    }

    // ── Categories ───────────────────────────────────────────────────────────
    fn categories(&self) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        for a in &self.accounts {
            if !a.category.is_empty() {
                seen.insert(a.category.clone());
            }
        }
        seen.into_iter().collect()
    }

    fn filtered_labels(&self) -> Vec<String> {
        let term = self.search.to_lowercase();
        self.accounts
            .iter()
            .filter(|a| {
                let cat_ok = if self.sel_cat.is_empty() {
                    true
                } else if self.sel_cat == "__none__" {
                    a.category.is_empty()
                } else {
                    a.category == self.sel_cat
                };
                if !cat_ok {
                    return false;
                }
                if term.is_empty() {
                    return true;
                }
                a.label.to_lowercase().contains(&term)
                    || a.issuer.to_lowercase().contains(&term)
                    || a.category.to_lowercase().contains(&term)
            })
            .map(|a| a.label.clone())
            .collect()
    }

    // ── Vault operations ─────────────────────────────────────────────────────
    fn do_reload(&mut self) -> Result<(), String> {
        if self.db_pass.is_empty() {
            return Err("Passphrase is required.".into());
        }
        self.accounts = load_accounts(&self.db_pb(), &self.db_pass).map_err(|e| e.to_string())?;
        self.accounts.sort_by(|a, b| {
            a.category
                .to_lowercase()
                .cmp(&b.category.to_lowercase())
                .then(a.issuer.to_lowercase().cmp(&b.issuer.to_lowercase()))
                .then(a.label.to_lowercase().cmp(&b.label.to_lowercase()))
        });
        self.accounts_loaded = true;
        self.persist_cfg();
        if !self.accounts.iter().any(|a| a.label == self.sel_label) {
            self.sel_label = self
                .accounts
                .first()
                .map(|a| a.label.clone())
                .unwrap_or_default();
        }
        Ok(())
    }

    fn do_save(&mut self) -> Result<(), String> {
        save_accounts(&self.db_pb(), &self.accounts, &self.db_pass).map_err(|e| e.to_string())
    }

    fn push_imported(&mut self, account: Account) -> Result<(), String> {
        if self.accounts.iter().any(|a| a.label == account.label) {
            return Err("An account with this label already exists.".into());
        }
        if let Some(existing) = label_for_secret(&self.accounts, &account.secret) {
            self.dup_pending = Some(account);
            return Err(format!(
                "This secret is already stored as \"{existing}\". Confirm to add anyway."
            ));
        }
        self.accounts.push(account);
        self.do_save()?;
        self.do_reload()?;
        self.panel = Panel::Accounts;
        Ok(())
    }

    fn commit_dup_pending(&mut self) -> Result<(), String> {
        let account = self.dup_pending.take().ok_or("No pending account.")?;
        if self.accounts.iter().any(|a| a.label == account.label) {
            return Err("An account with this label already exists.".into());
        }
        self.accounts.push(account);
        self.do_save()?;
        self.do_reload()?;
        self.panel = Panel::Accounts;
        Ok(())
    }

    fn do_add_manual(&mut self) -> Result<(), String> {
        if self.af_issuer.is_empty() || self.af_label.is_empty() || self.af_secret.is_empty() {
            return Err("Issuer, label, and secret are required.".into());
        }
        let secret = decode_secret(&self.af_secret).map_err(|e| e.to_string())?;
        validate_period(self.af_period).map_err(|e| e.to_string())?;
        validate_digits(self.af_digits).map_err(|e| e.to_string())?;
        self.do_reload()?;
        if self.accounts.iter().any(|a| a.label == self.af_label) {
            return Err("An account with this label already exists.".into());
        }
        let account = Account {
            issuer: self.af_issuer.clone(),
            label: self.af_label.clone(),
            secret,
            algorithm: self.af_algo,
            period_seconds: self.af_period,
            digits: self.af_digits,
            category: self.af_cat.clone(),
        };
        if let Some(existing) = label_for_secret(&self.accounts, &account.secret) {
            self.dup_pending = Some(account);
            return Err(format!(
                "This secret is already stored as \"{existing}\". Confirm to add anyway."
            ));
        }
        self.accounts.push(account);
        self.do_save()?;
        self.do_reload()?;
        self.panel = Panel::Accounts;
        self.af_issuer.clear();
        self.af_label.clear();
        self.af_secret.zeroize();
        self.af_algo = TotpAlgorithm::default();
        self.af_period = 30;
        self.af_digits = 6;
        self.af_cat.clear();
        Ok(())
    }

    fn do_import_uri(&mut self) -> Result<(), String> {
        if self.if_uri.is_empty() {
            return Err("OTP URI is required.".into());
        }
        let account = parse_otpauth_uri(&self.if_uri).map_err(|e| e.to_string())?;
        self.do_reload()?;
        let label = self.if_uri.clone();
        self.if_uri.clear();
        let _ = label;
        self.push_imported(account)
    }

    fn do_import_qr(&mut self) -> Result<(), String> {
        let cleaned = Self::clean_path(&self.if_qr);
        let path = PathBuf::from(&cleaned);
        if !path.exists() {
            return Err(format!("File not found: {cleaned}"));
        }
        let account = parse_otpauth_uri_from_qr_image(&path).map_err(|e| e.to_string())?;
        self.do_reload()?;
        self.if_qr.clear();
        self.push_imported(account)
    }

    fn do_import_camera(&mut self) -> Result<(), String> {
        let idx = self
            .if_cam
            .parse::<u32>()
            .map_err(|_| "Camera index must be a number (e.g. 0).".to_string())?;
        let fmt = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut cam = Camera::new(CameraIndex::Index(idx), fmt).map_err(|e| e.to_string())?;
        cam.open_stream().map_err(|e| e.to_string())?;
        let frame = cam.frame().map_err(|e| e.to_string())?;
        let rgb = frame
            .decode_image::<RgbFormat>()
            .map_err(|e| e.to_string())?;
        let gray = image::DynamicImage::ImageRgb8(rgb).to_luma8();
        let account = parse_otpauth_uri_from_luma(gray).map_err(|e| e.to_string())?;
        self.do_reload()?;
        self.push_imported(account)
    }

    fn do_update_account(&mut self) -> Result<(), String> {
        let orig = self
            .editing
            .clone()
            .ok_or("No account selected for editing.")?;
        if self.ef_label.trim().is_empty() || self.ef_issuer.trim().is_empty() {
            return Err("Issuer and label are required.".into());
        }
        validate_period(self.ef_period).map_err(|e| e.to_string())?;
        validate_digits(self.ef_digits).map_err(|e| e.to_string())?;
        self.do_reload()?;
        let idx = self
            .accounts
            .iter()
            .position(|a| a.label == orig)
            .ok_or("Account not found.")?;
        if self
            .accounts
            .iter()
            .enumerate()
            .any(|(i, a)| i != idx && a.label == self.ef_label)
        {
            return Err("Another account already uses this label.".into());
        }
        self.accounts[idx].issuer = self.ef_issuer.clone();
        self.accounts[idx].label = self.ef_label.clone();
        self.accounts[idx].algorithm = self.ef_algo;
        self.accounts[idx].period_seconds = self.ef_period;
        self.accounts[idx].digits = self.ef_digits;
        self.accounts[idx].category = self.ef_cat.clone();
        if !self.ef_secret.trim().is_empty() {
            self.accounts[idx].secret =
                decode_secret(&self.ef_secret).map_err(|e| e.to_string())?;
        }
        self.do_save()?;
        self.sel_label = self.ef_label.clone();
        self.editing = None;
        self.do_reload()?;
        Ok(())
    }

    fn do_delete_account(&mut self, label: String) -> Result<(), String> {
        self.do_reload()?;
        let before = self.accounts.len();
        self.accounts.retain(|a| a.label != label);
        if self.accounts.len() == before {
            return Err("Account not found.".into());
        }
        self.do_save()?;
        if self.sel_label == label {
            self.sel_label = self
                .accounts
                .first()
                .map(|a| a.label.clone())
                .unwrap_or_default();
        }
        self.do_reload()?;
        Ok(())
    }

    fn do_export_backup(&mut self) -> Result<(), String> {
        if self.bk_pass.is_empty() {
            return Err("Backup passphrase is required.".into());
        }
        export_backup(
            &self.db_pb(),
            &PathBuf::from(&self.bk_path),
            &self.db_pass,
            &self.bk_pass,
        )
        .map_err(|e| e.to_string())
    }

    fn do_import_backup(&mut self, replace: bool) -> Result<String, String> {
        if self.bk_pass.is_empty() {
            return Err("Backup passphrase is required.".into());
        }
        let result = import_backup(
            &PathBuf::from(&self.bk_path),
            &self.db_pb(),
            &self.bk_pass,
            &self.db_pass,
            replace,
        )
        .map_err(|e| e.to_string())?;
        self.do_reload()?;
        if result.replaced {
            Ok(format!("Vault replaced with {} account(s).", result.added))
        } else {
            let mut msg = format!(
                "Merged backup: {} added, {} skipped.",
                result.added,
                result.skipped_labels.len()
            );
            if !result.duplicate_secrets.is_empty() {
                msg.push_str(" Some added accounts share a secret with an existing one.");
            }
            Ok(msg)
        }
    }

    fn do_change_passphrase(&mut self) -> Result<(), String> {
        if !self.accounts_loaded {
            return Err("Load the vault before changing the passphrase.".into());
        }
        if self.new_pass.is_empty() {
            return Err("New passphrase cannot be empty.".into());
        }
        if self.new_pass != self.new_pass_confirm {
            return Err("New passphrase and confirmation do not match.".into());
        }
        change_passphrase(&self.db_pb(), &self.db_pass, &self.new_pass)
            .map_err(|e| e.to_string())?;
        self.db_pass = self.new_pass.clone();
        self.new_pass.zeroize();
        self.new_pass_confirm.zeroize();
        let _ = self.do_save_keychain();
        Ok(())
    }

    fn keychain_entry(&self) -> Result<Entry, String> {
        if self.db_path.trim().is_empty() {
            return Err("Database path is required.".into());
        }
        Entry::new("custom2fa.desktop", &self.db_path).map_err(|e| e.to_string())
    }

    fn do_save_keychain(&mut self) -> Result<(), String> {
        if self.db_pass.is_empty() {
            return Err("Passphrase is required.".into());
        }
        self.keychain_entry()?
            .set_password(&self.db_pass)
            .map_err(|e| e.to_string())
    }

    fn do_load_keychain(&mut self) -> Result<(), String> {
        self.db_pass = self
            .keychain_entry()?
            .get_password()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn do_clear_keychain(&mut self) -> Result<(), String> {
        self.keychain_entry()?
            .delete_credential()
            .map_err(|e| e.to_string())
    }

    fn begin_edit(&mut self, label: &str) {
        if let Some(a) = self.accounts.iter().find(|a| a.label == label) {
            self.editing = Some(label.to_string());
            self.ef_issuer = a.issuer.clone();
            self.ef_label = a.label.clone();
            self.ef_secret.zeroize();
            self.ef_algo = a.algorithm;
            self.ef_period = a.period_seconds;
            self.ef_digits = a.digits;
            self.ef_cat = a.category.clone();
        }
    }

    /// Scrubs decrypted secrets, passphrases, and cached codes from memory.
    fn zeroize_sensitive_memory(&mut self) {
        zeroize_accounts(&mut self.accounts);
        self.accounts.clear();

        for (_, (mut disp, mut raw, _, _)) in self.live_codes.drain() {
            disp.zeroize();
            raw.zeroize();
        }

        self.db_pass.zeroize();
        self.bk_pass.zeroize();
        self.af_secret.zeroize();
        self.ef_secret.zeroize();
        self.new_pass.zeroize();
        self.new_pass_confirm.zeroize();
        self.clipboard_value.zeroize();
    }

    fn lock_vault(&mut self) {
        self.zeroize_sensitive_memory();
        self.accounts_loaded = false;
        self.sel_label.clear();
        self.editing = None;
        self.del_label = None;
        self.expanded_labels.clear();
        self.revealed_labels.clear();
        self.dup_pending = None;
        self.backup_prompt = false;
        self.new_pass.zeroize();
        self.new_pass_confirm.zeroize();
    }

    // ── Sidebar ──────────────────────────────────────────────────────────────
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.label(
            RichText::new("Custom2FA")
                .size(22.0)
                .strong()
                .color(C_ACCENT),
        );
        ui.label(RichText::new("Offline TOTP Manager").small().color(C_MUTED));
        ui.add_space(6.0);

        if self.accounts_loaded {
            ui.label(
                RichText::new(format!("Vault open  ·  {} accounts", self.accounts.len()))
                    .small()
                    .color(C_OK),
            );
        } else {
            ui.label(RichText::new("Vault locked").small().color(C_MUTED));
        }

        ui.separator();

        // Search
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search accounts…  Ctrl+F")
                .desired_width(f32::INFINITY)
                .id(egui::Id::new("search_box")),
        );
        if self.search_focus {
            search.request_focus();
            self.search_focus = false;
        }

        ui.add_space(6.0);

        // Category filter (only when accounts are loaded)
        if self.accounts_loaded && !self.accounts.is_empty() {
            let total = self.accounts.len();
            let cats = self.categories();
            let none_count = self
                .accounts
                .iter()
                .filter(|a| a.category.is_empty())
                .count();

            if ui
                .selectable_label(
                    self.sel_cat.is_empty(),
                    RichText::new(format!("All  ({total})")).color(C_TEXT),
                )
                .clicked()
            {
                self.sel_cat.clear();
                self.panel = Panel::Accounts;
            }

            for cat in &cats {
                let n = self.accounts.iter().filter(|a| &a.category == cat).count();
                let sel = self.sel_cat == *cat;
                if ui
                    .selectable_label(sel, RichText::new(format!("  {cat}  ({n})")).color(C_TEXT))
                    .clicked()
                {
                    self.sel_cat = cat.clone();
                    self.panel = Panel::Accounts;
                }
            }

            if none_count > 0 && !cats.is_empty() {
                let sel = self.sel_cat == "__none__";
                if ui
                    .selectable_label(
                        sel,
                        RichText::new(format!("  Uncategorised  ({none_count})")).color(C_MUTED),
                    )
                    .clicked()
                {
                    self.sel_cat = "__none__".into();
                    self.panel = Panel::Accounts;
                }
            }

            ui.separator();
        }

        ui.add_space(4.0);

        // Nav buttons
        for (p, label, color) in [
            (Panel::Accounts, "Accounts", C_TEXT),
            (Panel::Add, "+ Add / Import", C_ACCENT),
            (Panel::Backup, "Backup / Restore", C_TEXT),
            (Panel::Settings, "Settings", C_TEXT),
        ] {
            if ui
                .selectable_label(self.panel == p, RichText::new(label).color(color))
                .clicked()
            {
                self.panel = p;
            }
        }

        // Lock button pinned to bottom
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(8.0);
            if self.accounts_loaded
                && ui
                    .button(RichText::new("Lock Vault").color(C_ERR))
                    .clicked()
            {
                self.lock_vault();
                self.set_ok("Vault locked.");
            }
            ui.separator();
        });
    }

    // ── Accounts panel ───────────────────────────────────────────────────────
    fn show_accounts_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if !self.accounts_loaded {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("Vault is locked").size(18.0).color(C_MUTED));
                ui.add_space(6.0);
                ui.label(RichText::new("Open Settings to load your database.").color(C_MUTED));
                ui.add_space(16.0);
                if ui.button("Open Settings").clicked() {
                    self.panel = Panel::Settings;
                }
            });
            return;
        }

        let labels = self.filtered_labels();
        if labels.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("No accounts match your filter.").color(C_MUTED));
            });
            return;
        }

        // Collect display data — borrows nothing from self after this point
        let cards: Vec<CardData> = labels
            .iter()
            .filter_map(|lbl| {
                let acc = self.accounts.iter().find(|a| &a.label == lbl)?;
                let (disp, raw, secs, frac) = self
                    .live_codes
                    .get(lbl)
                    .cloned()
                    .unwrap_or_else(|| ("------".into(), "------".into(), 0, 0.0));
                Some(CardData {
                    issuer: acc.issuer.clone(),
                    label: acc.label.clone(),
                    category: acc.category.clone(),
                    algo: acc.algorithm.to_string(),
                    digits: acc.digits,
                    period: acc.period_seconds,
                    code: disp,
                    raw_code: raw,
                    secs,
                    frac,
                    expanded: self.expanded_labels.contains(lbl),
                    hidden: self.cfg.hide_codes && !self.revealed_labels.contains(lbl),
                })
            })
            .collect();

        let mut pending: Option<CardAction> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(8.0);
                for card in &cards {
                    show_card(ui, card, &mut pending);
                    ui.add_space(6.0);
                }
                ui.add_space(8.0);
            });

        match pending {
            Some(CardAction::Edit(lbl)) => self.begin_edit(&lbl),
            Some(CardAction::Delete(lbl)) => self.del_label = Some(lbl),
            Some(CardAction::Copied { label, code }) => {
                self.copy_code(ctx, &label, code);
            }
            Some(CardAction::Toggle(lbl)) => {
                if self.expanded_labels.contains(&lbl) {
                    self.expanded_labels.remove(&lbl);
                } else {
                    self.expanded_labels.insert(lbl);
                }
            }
            None => {}
        }
    }

    // ── Add / Import panel ───────────────────────────────────────────────────
    fn show_add_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("Add / Import Account")
                .size(18.0)
                .strong()
                .color(C_ACCENT),
        );
        ui.add_space(8.0);

        // Tab bar
        ui.horizontal(|ui| {
            for (tab, label) in [
                (AddTab::Manual, "Manual Secret"),
                (AddTab::Uri, "OTP URI"),
                (AddTab::QrImage, "QR Image"),
                (AddTab::Camera, "Camera"),
            ] {
                let sel = self.add_tab == tab;
                let txt = if sel {
                    RichText::new(label).color(C_ACCENT).strong()
                } else {
                    RichText::new(label).color(C_MUTED)
                };
                if ui.selectable_label(sel, txt).clicked() {
                    self.add_tab = tab;
                }
            }
        });

        ui.separator();
        ui.add_space(8.0);

        match self.add_tab {
            AddTab::Manual => {
                form_row(ui, "Issuer", &mut self.af_issuer, false);
                form_row(ui, "Label", &mut self.af_label, false);
                form_row(ui, "Base32 secret", &mut self.af_secret, true);
                form_row(ui, "Category (optional)", &mut self.af_cat, false);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Algorithm").color(C_MUTED).small());
                    ui.add_space(4.0);
                    algo_combo(ui, "af_algo", &mut self.af_algo);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Period").color(C_MUTED).small());
                    ui.add_space(4.0);
                    period_combo(ui, "af_period", &mut self.af_period);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Digits").color(C_MUTED).small());
                    ui.add_space(4.0);
                    digits_combo(ui, "af_digits", &mut self.af_digits);
                });
                ui.add_space(12.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("Add Account").color(C_ACCENT))
                            .min_size([200.0, 32.0].into()),
                    )
                    .clicked()
                {
                    self.exec("Account added.", |s| s.do_add_manual());
                }
            }
            AddTab::Uri => {
                ui.label(RichText::new("Paste the full otpauth://totp/… URI:").color(C_MUTED));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.if_uri)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("otpauth://totp/Label?secret=…&issuer=…"),
                );
                ui.add_space(8.0);
                if ui
                    .add(egui::Button::new("Import URI").min_size([200.0, 32.0].into()))
                    .clicked()
                {
                    self.exec("Account imported from URI.", |s| s.do_import_uri());
                }
            }
            AddTab::QrImage => {
                ui.label(RichText::new("PNG or JPG QR code image:").color(C_MUTED));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.if_qr)
                            .desired_width(ui.available_width() - 90.0)
                            .hint_text("Select a QR image…"),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(p) =
                            Self::pick_file(&[("Images", &["png", "jpg", "jpeg", "webp"])])
                        {
                            self.if_qr = p;
                        }
                    }
                });
                ui.label(
                    RichText::new("Surrounding quotes are stripped automatically.")
                        .small()
                        .color(C_MUTED),
                );
                ui.add_space(8.0);
                if ui
                    .add(egui::Button::new("Import QR Image").min_size([200.0, 32.0].into()))
                    .clicked()
                {
                    self.exec("Account imported from QR image.", |s| s.do_import_qr());
                }
            }
            AddTab::Camera => {
                ui.label(
                    RichText::new("Show the QR code to your webcam, then click Scan.")
                        .color(C_MUTED),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Camera index").color(C_MUTED).small());
                    ui.add_space(4.0);
                    ui.add(egui::TextEdit::singleline(&mut self.if_cam).desired_width(60.0));
                    ui.label(RichText::new("(0 = first camera)").small().color(C_MUTED));
                });
                ui.add_space(8.0);
                if ui
                    .add(egui::Button::new("Scan QR From Camera").min_size([200.0, 32.0].into()))
                    .clicked()
                {
                    self.exec("Account imported from camera.", |s| s.do_import_camera());
                }
            }
        }
    }

    // ── Backup panel ─────────────────────────────────────────────────────────
    fn show_backup_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("Backup / Restore")
                .size(18.0)
                .strong()
                .color(C_ACCENT),
        );
        ui.add_space(12.0);

        ui.label(RichText::new("Backup file path:").color(C_MUTED).small());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.bk_path)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text("backup-2fa.json"),
            );
            if ui.button("Browse…").clicked() {
                if let Some(p) = Self::save_file(&[("Backup JSON", &["json"])], "backup-2fa.json") {
                    self.bk_path = p;
                }
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("Backup passphrase (separate from your vault passphrase):")
                .color(C_MUTED)
                .small(),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.bk_pass)
                .password(true)
                .desired_width(f32::INFINITY)
                .hint_text("Enter a backup passphrase…"),
        );
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Export Backup").min_size([170.0, 32.0].into()))
                .clicked()
            {
                self.exec("Backup exported successfully.", |s| s.do_export_backup());
            }
            ui.add_space(8.0);
            if ui
                .add(egui::Button::new("Import Backup").min_size([170.0, 32.0].into()))
                .clicked()
            {
                self.backup_prompt = true;
            }
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("Notes").strong().color(C_ACCENT));
        ui.add_space(4.0);
        ui.label("The backup file is independently encrypted with the backup passphrase.");
        ui.label("Store backups in a location separate from your primary device.");
        ui.label("If you lose both the vault and the backup, accounts cannot be recovered.");
    }

    // ── Settings panel ───────────────────────────────────────────────────────
    fn show_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("Settings")
                .size(18.0)
                .strong()
                .color(C_ACCENT),
        );
        ui.add_space(14.0);

        ui.label(RichText::new("Database file path:").color(C_MUTED).small());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.db_path)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text("accounts.c2fa"),
            );
            if ui.button("Browse…").clicked() {
                if let Some(p) = Self::save_file(&[("Vault", &["c2fa"])], "accounts.c2fa") {
                    self.db_path = p;
                    self.persist_cfg();
                }
            }
        });
        ui.add_space(8.0);

        ui.label(RichText::new("Database passphrase:").color(C_MUTED).small());
        ui.add(
            egui::TextEdit::singleline(&mut self.db_pass)
                .password(true)
                .desired_width(f32::INFINITY)
                .hint_text("Enter passphrase…"),
        );
        ui.add_space(12.0);

        ui.label(
            RichText::new("OS Keychain (optional):")
                .color(C_MUTED)
                .small(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("Save to keychain").clicked() {
                self.exec("Passphrase saved to OS keychain.", |s| s.do_save_keychain());
            }
            if ui.button("Load from keychain").clicked() {
                self.exec("Passphrase loaded from OS keychain.", |s| {
                    s.do_load_keychain()
                });
            }
            if ui.button("Clear keychain entry").clicked() {
                self.exec("Keychain entry cleared.", |s| s.do_clear_keychain());
            }
        });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("Privacy").strong().color(C_ACCENT));
        ui.add_space(6.0);
        if ui
            .checkbox(&mut self.cfg.hide_codes, "Hide codes until clicked")
            .changed()
        {
            self.persist_cfg();
            if !self.cfg.hide_codes {
                self.revealed_labels.clear();
            }
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Idle auto-lock").color(C_MUTED).small());
            let mut secs = self.cfg.auto_lock_seconds;
            egui::ComboBox::from_id_salt("auto_lock")
                .selected_text(auto_lock_label(secs))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut secs, 0, "Off");
                    ui.selectable_value(&mut secs, 60, "1 minute");
                    ui.selectable_value(&mut secs, 300, "5 minutes");
                    ui.selectable_value(&mut secs, 900, "15 minutes");
                });
            if secs != self.cfg.auto_lock_seconds {
                self.cfg.auto_lock_seconds = secs;
                self.persist_cfg();
            }
        });
        if ui
            .checkbox(&mut self.cfg.auto_unlock, "Unlock from keychain on launch")
            .changed()
        {
            self.persist_cfg();
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(12.0);

        if ui
            .add(
                egui::Button::new(RichText::new("Load Accounts").color(C_ACCENT))
                    .min_size([f32::INFINITY, 34.0].into()),
            )
            .clicked()
        {
            self.exec("Vault loaded.", |s| s.do_reload());
            if self.accounts_loaded {
                self.panel = Panel::Accounts;
            }
        }

        if self.accounts_loaded {
            ui.add_space(6.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("Lock Vault").color(C_ERR))
                        .min_size([f32::INFINITY, 34.0].into()),
                )
                .clicked()
            {
                self.lock_vault();
                self.set_ok("Vault locked.");
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("Change vault passphrase")
                    .strong()
                    .color(C_ACCENT),
            );
            ui.add_space(6.0);
            form_row(ui, "New passphrase", &mut self.new_pass, true);
            form_row(
                ui,
                "Confirm new passphrase",
                &mut self.new_pass_confirm,
                true,
            );
            if ui.button("Change passphrase").clicked() {
                self.exec("Vault passphrase changed.", |s| s.do_change_passphrase());
            }
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("Shortcuts").strong().color(C_ACCENT));
        ui.add_space(4.0);
        ui.label("Ctrl+F  search   ·   Ctrl+N  add   ·   Ctrl+L  lock   ·   Esc  close dialog");
        ui.add_space(8.0);
        ui.label(RichText::new("Notes").strong().color(C_ACCENT));
        ui.add_space(4.0);
        ui.label("The passphrase is only held in memory — never written to disk in plaintext.");
        ui.label("The OS keychain entry is stored per-user and does not sync across machines.");
        ui.label("Copied codes are cleared from the clipboard after 30 seconds.");
        ui.label("To use an existing vault from another machine, copy the .c2fa file here.");
    }

    // ── Edit account window ──────────────────────────────────────────────────
    fn show_edit_window(&mut self, ctx: &egui::Context) {
        if self.editing.is_none() {
            return;
        }

        let mut do_save = false;
        let mut do_cancel = false;

        egui::Window::new("Edit Account")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                form_row(ui, "Issuer", &mut self.ef_issuer, false);
                form_row(ui, "Label", &mut self.ef_label, false);
                form_row(ui, "Category", &mut self.ef_cat, false);
                ui.add_space(2.0);
                ui.label(
                    RichText::new("New Base32 secret (leave blank to keep existing):")
                        .small()
                        .color(C_MUTED),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.ef_secret)
                        .password(true)
                        .hint_text("Leave blank to keep current secret")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Algorithm").color(C_MUTED).small());
                    ui.add_space(4.0);
                    algo_combo(ui, "ef_algo", &mut self.ef_algo);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Period").color(C_MUTED).small());
                    ui.add_space(4.0);
                    period_combo(ui, "ef_period", &mut self.ef_period);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Digits").color(C_MUTED).small());
                    ui.add_space(4.0);
                    digits_combo(ui, "ef_digits", &mut self.ef_digits);
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("Save Changes").min_size([150.0, 30.0].into()))
                        .clicked()
                    {
                        do_save = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Cancel").color(C_MUTED))
                                .min_size([80.0, 30.0].into()),
                        )
                        .clicked()
                    {
                        do_cancel = true;
                    }
                });
            });

        if do_save {
            self.exec("Account updated.", |s| s.do_update_account());
        } else if do_cancel {
            self.editing = None;
        }
    }

    // ── Delete confirmation window ───────────────────────────────────────────
    fn show_delete_confirm(&mut self, ctx: &egui::Context) {
        let label = match self.del_label.clone() {
            Some(l) => l,
            None => return,
        };

        let mut do_delete = false;
        let mut do_cancel = false;

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label(RichText::new("Delete account?").size(16.0).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(format!("\"{}\"", label)).color(C_TEXT));
                ui.add_space(4.0);
                ui.label(RichText::new("This cannot be undone.").small().color(C_ERR));
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Delete").color(C_ERR))
                                .min_size([120.0, 30.0].into()),
                        )
                        .clicked()
                    {
                        do_delete = true;
                    }
                    if ui
                        .add(egui::Button::new("Cancel").min_size([80.0, 30.0].into()))
                        .clicked()
                    {
                        do_cancel = true;
                    }
                });
            });

        if do_delete {
            self.del_label = None;
            self.exec("Account deleted.", |s| s.do_delete_account(label));
        } else if do_cancel {
            self.del_label = None;
        }
    }

    fn show_backup_prompt(&mut self, ctx: &egui::Context) {
        if !self.backup_prompt {
            return;
        }
        let mut merge = false;
        let mut replace = false;
        let mut cancel = false;
        egui::Window::new("Import Backup")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                ui.label(RichText::new("How should this backup be applied?").strong());
                ui.add_space(6.0);
                ui.label("Merge keeps existing accounts and skips duplicate labels.");
                ui.label(
                    RichText::new("Replace overwrites the entire vault.")
                        .color(C_ERR)
                        .small(),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("Merge").min_size([110.0, 30.0].into()))
                        .clicked()
                    {
                        merge = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Replace vault").color(C_ERR))
                                .min_size([130.0, 30.0].into()),
                        )
                        .clicked()
                    {
                        replace = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if merge || replace {
            self.backup_prompt = false;
            match self.do_import_backup(replace) {
                Ok(msg) => self.set_ok(msg),
                Err(e) => self.set_err(e),
            }
        } else if cancel {
            self.backup_prompt = false;
        }
    }

    fn show_dup_confirm(&mut self, ctx: &egui::Context) {
        if self.dup_pending.is_none() {
            return;
        }
        let label = self
            .dup_pending
            .as_ref()
            .map(|a| a.label.clone())
            .unwrap_or_default();
        let mut add = false;
        let mut cancel = false;
        egui::Window::new("Duplicate Secret")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label("This secret is already stored on another account.");
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Add \"{label}\" anyway?")).color(C_TEXT));
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("Add anyway").clicked() {
                        add = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if add {
            self.exec("Account added.", |s| s.commit_dup_pending());
        } else if cancel {
            self.dup_pending = None;
        }
    }

    // ── Status bar ───────────────────────────────────────────────────────────
    fn show_status_bar(&self, ui: &mut egui::Ui) {
        if self.status.is_empty() {
            ui.label(RichText::new(" ").small());
            return;
        }
        let (icon, color) = if self.is_err {
            ("⚠  ", C_ERR)
        } else {
            ("✔  ", C_OK)
        };
        ui.label(
            RichText::new(format!("{icon}{}", self.status))
                .small()
                .color(color),
        );
    }
}

// ── Free helper functions ─────────────────────────────────────────────────────

/// Render a single account card. Uses `pending` to report user actions without
/// needing access to &mut app state inside the rendering closure.
///
/// Layout (always-visible header):
///   [▶/▼ ISSUER]  ···spacer···  [123 456]  [████░ Xs]  [Copy]
///
/// Expanded section (shown below header when card.expanded == true):
///   label · category · algo/period/digits meta · [Edit] [Delete]
fn show_card(ui: &mut egui::Ui, card: &CardData, pending: &mut Option<CardAction>) {
    let bar_color = if card.secs > 10 {
        C_OK
    } else if card.secs > 5 {
        C_WARN
    } else {
        C_ERR
    };

    let frame = egui::Frame {
        fill: C_CARD,
        stroke: Stroke::new(1.0, C_BORDER),
        corner_radius: egui::CornerRadius::same(8),
        inner_margin: egui::Margin::same(10),
        ..Default::default()
    };

    frame.show(ui, |ui| {
        // `ui.available_width()` here is already the *inner* width after
        // inner_margin (10 px each side) and stroke (1 px each side) are
        // subtracted by egui's frame layout.  Setting set_min_width to this
        // value forces the card to fill the available space exactly without
        // overflowing or leaving a gap.
        ui.set_min_width(ui.available_width());

        // ── Always-visible header row ────────────────────────────────────
        // Left section: chevron + issuer name (expand/collapse toggle).
        // Right section (RTL): [Copy] [timer label] [progress bar] [code].
        // The code button has an explicit min_size so the text can never
        // wrap digit-by-digit regardless of layout width.
        ui.horizontal(|ui| {
            // Expand / collapse toggle
            let chevron = if card.expanded { "▼  " } else { "▶  " };
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(format!("{}{}", chevron, card.issuer))
                            .size(14.0)
                            .strong()
                            .color(C_ACCENT),
                    )
                    .frame(false),
                )
                .on_hover_text(if card.expanded { "Collapse" } else { "Expand" })
                .clicked()
            {
                *pending = Some(CardAction::Toggle(card.label.clone()));
            }

            // Right side — RTL so items stack: Copy | Xs | ProgressBar | Code
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 1. Copy button (rightmost)
                if ui
                    .add(egui::Button::new("Copy").min_size([50.0, 26.0].into()))
                    .clicked()
                {
                    *pending = Some(CardAction::Copied {
                        label: card.label.clone(),
                        code: card.raw_code.clone(),
                    });
                }

                // 2. Countdown timer label
                ui.label(
                    RichText::new(format!("  {}s  ", card.secs))
                        .small()
                        .color(C_MUTED),
                );

                // 3. Progress bar
                ui.add(
                    egui::ProgressBar::new(1.0 - card.frac)
                        .fill(bar_color)
                        .desired_width(90.0),
                );

                ui.add_space(10.0);

                // 4. Code — min_size guarantees the text is never squeezed
                //    to sub-character width by the surrounding RTL layout.
                let shown = if card.hidden {
                    match card.digits {
                        6 => "••• •••".to_string(),
                        8 => "•••• ••••".to_string(),
                        n => "•".repeat(n as usize),
                    }
                } else {
                    card.code.clone()
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(&shown)
                                .monospace()
                                .size(22.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .frame(false)
                        .min_size([130.0, 34.0].into()),
                    )
                    .on_hover_text(if card.hidden {
                        "Click to reveal and copy"
                    } else {
                        "Click to copy"
                    })
                    .clicked()
                {
                    *pending = Some(CardAction::Copied {
                        label: card.label.clone(),
                        code: card.raw_code.clone(),
                    });
                }
            });
        });

        // ── Expanded detail section ──────────────────────────────────────
        if card.expanded {
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // Indent the detail content slightly
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&card.label).size(12.0).color(C_TEXT));

                    if !card.category.is_empty() {
                        ui.label(
                            RichText::new(format!("Category:  {}", card.category))
                                .small()
                                .color(C_MUTED),
                        );
                    }

                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {} digits  ·  {}s period",
                            card.algo, card.digits, card.period
                        ))
                        .small()
                        .color(C_MUTED),
                    );

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Edit").min_size([72.0, 26.0].into()))
                            .clicked()
                        {
                            *pending = Some(CardAction::Edit(card.label.clone()));
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Delete").color(C_ERR))
                                    .min_size([72.0, 26.0].into()),
                            )
                            .clicked()
                        {
                            *pending = Some(CardAction::Delete(card.label.clone()));
                        }
                    });
                });
            });
        }
    });
}

/// Labelled form row with a full-width text field.
fn form_row(ui: &mut egui::Ui, label: &str, value: &mut String, password: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(C_MUTED));
    });
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .password(password),
    );
    ui.add_space(2.0);
}

fn algo_combo(ui: &mut egui::Ui, id: &str, algo: &mut TotpAlgorithm) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(algo.to_string())
        .show_ui(ui, |ui| {
            ui.selectable_value(algo, TotpAlgorithm::Sha1, "SHA-1 (default)");
            ui.selectable_value(algo, TotpAlgorithm::Sha256, "SHA-256");
            ui.selectable_value(algo, TotpAlgorithm::Sha512, "SHA-512");
        });
}

fn period_combo(ui: &mut egui::Ui, id: &str, period: &mut u32) {
    let mut options = vec![15u32, 30, 60, 90];
    if !options.contains(period) {
        options.push(*period);
        options.sort_unstable();
    }
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("{period} seconds"))
        .show_ui(ui, |ui| {
            for p in options {
                ui.selectable_value(period, p, format!("{p} seconds"));
            }
        });
}

fn digits_combo(ui: &mut egui::Ui, id: &str, digits: &mut u8) {
    let mut options = vec![6u8, 7, 8];
    if !options.contains(digits) {
        options.push(*digits);
        options.sort_unstable();
    }
    egui::ComboBox::from_id_salt(id)
        .selected_text(digits.to_string())
        .show_ui(ui, |ui| {
            for d in options {
                ui.selectable_value(digits, d, format!("{d} digits"));
            }
        });
}

fn auto_lock_label(secs: u32) -> &'static str {
    match secs {
        0 => "Off",
        60 => "1 minute",
        300 => "5 minutes",
        900 => "15 minutes",
        _ => "Custom",
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────
impl Drop for Custom2faApp {
    fn drop(&mut self) {
        self.zeroize_sensitive_memory();
    }
}

impl eframe::App for Custom2faApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_cfg();
        self.zeroize_sensitive_memory();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(250));
        self.note_input(ctx);
        self.handle_shortcuts(ctx);
        self.maybe_auto_lock();
        self.maybe_clear_clipboard();

        let size = ctx.input(|i| i.content_rect().size());
        if (size.x - self.cfg.window_width).abs() > 1.0
            || (size.y - self.cfg.window_height).abs() > 1.0
        {
            self.cfg.window_width = size.x;
            self.cfg.window_height = size.y;
        }

        if self.accounts_loaded {
            self.refresh_live_codes();
        }

        if self.editing.is_some() {
            self.show_edit_window(ctx);
        }
        if self.del_label.is_some() {
            self.show_delete_confirm(ctx);
        }
        if self.backup_prompt {
            self.show_backup_prompt(ctx);
        }
        if self.dup_pending.is_some() {
            self.show_dup_confirm(ctx);
        }

        // Status bar (bottom)
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(26.0)
            .show(ctx, |ui| {
                ui.add_space(3.0);
                self.show_status_bar(ui);
            });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .min_width(210.0)
            .max_width(270.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_sidebar(ui);
                });
            });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| match self.panel {
            Panel::Accounts => self.show_accounts_panel(ui, ctx),
            Panel::Add => self.show_add_panel(ui),
            Panel::Backup => self.show_backup_panel(ui),
            Panel::Settings => self.show_settings_panel(ui),
        });
    }
}

// ── App icon (programmatic, no file required) ─────────────────────────────────
/// Generates a 64×64 RGBA icon: dark circle with an accent-blue ring border
/// and a pixelated "2" glyph rendered in a lighter blue at the centre.
fn app_icon() -> egui::IconData {
    const S: u32 = 64;
    let c = S as f32 / 2.0;
    let r = c - 1.5_f32; // outer radius (leave 1-px breathing room)
    let ring_w = 9.0_f32; // width of the blue ring border

    // 5-wide × 7-tall bitmap for the digit "2"
    let glyph: [[u8; 5]; 7] = [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [0, 0, 0, 1, 0],
        [0, 0, 1, 1, 0],
        [0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ];
    let scale = 4.2_f32;
    let gw = 5.0_f32 * scale;
    let gh = 7.0_f32 * scale;
    let gx0 = c - gw / 2.0;
    let gy0 = c - gh / 2.0 + 1.5; // very slight downward nudge

    let mut rgba = vec![0u8; (S * S * 4) as usize];

    for y in 0..S {
        for x in 0..S {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - c;
            let dy = py - c;
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * S + x) * 4) as usize;

            if d > r {
                // soft anti-aliased edge
                if d <= r + 1.0 {
                    let t = (r + 1.0 - d).clamp(0.0, 1.0);
                    rgba[i] = (26.0 * t) as u8;
                    rgba[i + 1] = (26.0 * t) as u8;
                    rgba[i + 2] = (38.0 * t) as u8;
                    rgba[i + 3] = (255.0 * t) as u8;
                }
                continue;
            }

            if d >= r - ring_w {
                // accent-blue ring  (#7AA2F7)
                rgba[i] = 122;
                rgba[i + 1] = 162;
                rgba[i + 2] = 247;
                rgba[i + 3] = 255;
            } else {
                // dark inner background (#1A1A26)
                rgba[i] = 26;
                rgba[i + 1] = 26;
                rgba[i + 2] = 38;
                rgba[i + 3] = 255;

                // draw "2" glyph (lighter accent #C0CAF5)
                let gxi = ((px - gx0) / scale) as isize;
                let gyi = ((py - gy0) / scale) as isize;
                if (0..5).contains(&gxi)
                    && (0..7).contains(&gyi)
                    && glyph[gyi as usize][gxi as usize] == 1
                {
                    rgba[i] = 192;
                    rgba[i + 1] = 202;
                    rgba[i + 2] = 245;
                    rgba[i + 3] = 255;
                }
            }
        }
    }

    egui::IconData {
        rgba,
        width: S,
        height: S,
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() -> eframe::Result<()> {
    let cfg = config::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Custom2FA")
            .with_inner_size([cfg.window_width, cfg.window_height])
            .with_min_inner_size([720.0, 520.0])
            .with_icon(std::sync::Arc::new(app_icon())),
        ..Default::default()
    };
    eframe::run_native(
        "Custom2FA",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(build_visuals());
            let mut app = Custom2faApp::from_config(cfg);
            app.try_auto_unlock();
            Ok(Box::new(app))
        }),
    )
}
