use custom2fa_core::account::{validate_digits, validate_period, Account, TotpAlgorithm};
use custom2fa_core::otp_uri::{
    parse_otpauth_uri, parse_otpauth_uri_from_luma, parse_otpauth_uri_from_qr_image,
};
use custom2fa_core::storage::{export_backup, import_backup, load_accounts, save_accounts};
use custom2fa_core::totp::{decode_secret, format_totp_code, generate_totp_for_account};
use eframe::egui;
use keyring::Entry;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Custom2FA Hub",
        options,
        Box::new(|_cc| Ok(Box::<Custom2faApp>::default())),
    )
}

struct Custom2faApp {
    db_path: String,
    db_passphrase: String,
    search_term: String,
    issuer: String,
    label: String,
    secret: String,
    add_algorithm: TotpAlgorithm,
    add_period: String,
    add_digits: String,
    edit_issuer: String,
    edit_label: String,
    edit_secret: String,
    edit_algorithm: TotpAlgorithm,
    edit_period: String,
    edit_digits: String,
    uri: String,
    qr_image_path: String,
    camera_index: String,
    backup_path: String,
    backup_passphrase: String,
    selected_label: String,
    generated_code: String,
    status: String,
    accounts: Vec<Account>,
}

impl Default for Custom2faApp {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            db_passphrase: String::new(),
            search_term: String::new(),
            issuer: String::new(),
            label: String::new(),
            secret: String::new(),
            add_algorithm: TotpAlgorithm::default(),
            add_period: "30".to_string(),
            add_digits: "6".to_string(),
            edit_issuer: String::new(),
            edit_label: String::new(),
            edit_secret: String::new(),
            edit_algorithm: TotpAlgorithm::default(),
            edit_period: "30".to_string(),
            edit_digits: "6".to_string(),
            uri: String::new(),
            qr_image_path: String::new(),
            camera_index: "0".to_string(),
            backup_path: String::new(),
            backup_passphrase: String::new(),
            selected_label: String::new(),
            generated_code: String::new(),
            status: String::new(),
            accounts: Vec::new(),
        }
    }
}

impl Custom2faApp {
    fn clean_path_input(raw: &str) -> String {
        raw.trim().trim_matches('"').trim().to_string()
    }

    fn ensure_defaults(&mut self) {
        if self.db_path.is_empty() {
            self.db_path = "accounts.c2fa".to_string();
        }
        if self.backup_path.is_empty() {
            self.backup_path = "backup-2fa.json".to_string();
        }
        if self.camera_index.is_empty() {
            self.camera_index = "0".to_string();
        }
    }

    fn db_pathbuf(&self) -> PathBuf {
        PathBuf::from(self.db_path.clone())
    }

    fn reload_accounts(&mut self) -> Result<(), String> {
        if self.db_passphrase.is_empty() {
            return Err("Database passphrase is required.".to_string());
        }
        self.accounts = load_accounts(&self.db_pathbuf(), &self.db_passphrase).map_err(|e| e.to_string())?;
        self.accounts.sort_by_key(|a| a.label.to_lowercase());
        if self.accounts.is_empty() {
            self.selected_label.clear();
            self.edit_issuer.clear();
            self.edit_label.clear();
            self.edit_secret.clear();
            self.edit_algorithm = TotpAlgorithm::default();
            self.edit_period = "30".to_string();
            self.edit_digits = "6".to_string();
        } else if self.selected_label.is_empty()
            || !self.accounts.iter().any(|a| a.label == self.selected_label)
        {
            self.selected_label = self.accounts[0].label.clone();
            self.sync_edit_fields_from_selection();
        }
        Ok(())
    }

    fn filtered_accounts(&self) -> Vec<&Account> {
        if self.search_term.trim().is_empty() {
            return self.accounts.iter().collect();
        }
        let term = self.search_term.to_lowercase();
        self.accounts
            .iter()
            .filter(|a| {
                a.label.to_lowercase().contains(&term) || a.issuer.to_lowercase().contains(&term)
            })
            .collect()
    }

    fn sync_edit_fields_from_selection(&mut self) {
        if let Some(account) = self.accounts.iter().find(|a| a.label == self.selected_label) {
            self.edit_issuer = account.issuer.clone();
            self.edit_label = account.label.clone();
            self.edit_secret.clear();
            self.edit_algorithm = account.algorithm;
            self.edit_period = account.period_seconds.to_string();
            self.edit_digits = account.digits.to_string();
        }
    }

    fn add_manual(&mut self) -> Result<(), String> {
        if self.issuer.is_empty() || self.label.is_empty() || self.secret.is_empty() {
            return Err("Issuer, label, and secret are required.".to_string());
        }
        let secret_bytes = decode_secret(&self.secret).map_err(|e| e.to_string())?;
        let period_seconds = self
            .add_period
            .trim()
            .parse::<u32>()
            .map_err(|_| "Period must be a positive number (seconds).".to_string())?;
        let digits = self
            .add_digits
            .trim()
            .parse::<u8>()
            .map_err(|_| "Digits must be a number (typically 6 or 8).".to_string())?;
        validate_period(period_seconds).map_err(|e| e.to_string())?;
        validate_digits(digits).map_err(|e| e.to_string())?;
        self.reload_accounts()?;
        if self.accounts.iter().any(|a| a.label == self.label) {
            return Err("An account with this label already exists.".to_string());
        }
        self.accounts.push(Account {
            issuer: self.issuer.clone(),
            label: self.label.clone(),
            secret: secret_bytes,
            algorithm: self.add_algorithm,
            period_seconds,
            digits,
        });
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn import_uri(&mut self) -> Result<(), String> {
        if self.uri.is_empty() {
            return Err("OTP URI is required.".to_string());
        }
        let account = parse_otpauth_uri(&self.uri).map_err(|e| e.to_string())?;
        self.reload_accounts()?;
        if self.accounts.iter().any(|a| a.label == account.label) {
            return Err("An account with this label already exists.".to_string());
        }
        self.accounts.push(account);
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn import_qr(&mut self) -> Result<(), String> {
        if self.qr_image_path.is_empty() {
            return Err("QR image path is required.".to_string());
        }

        let cleaned = Self::clean_path_input(&self.qr_image_path);
        if cleaned.is_empty() {
            return Err("QR image path is required.".to_string());
        }
        let path = PathBuf::from(cleaned.clone());
        if !path.exists() {
            return Err(format!("QR image file was not found: {cleaned}"));
        }

        let account = parse_otpauth_uri_from_qr_image(&path)
            .map_err(|e| e.to_string())?;
        self.reload_accounts()?;
        if self.accounts.iter().any(|a| a.label == account.label) {
            return Err("An account with this label already exists.".to_string());
        }
        self.accounts.push(account);
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn import_qr_from_camera(&mut self) -> Result<(), String> {
        let index = self
            .camera_index
            .parse::<u32>()
            .map_err(|_| "Camera index must be a number (ex: 0).".to_string())?;
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut camera =
            Camera::new(CameraIndex::Index(index), requested).map_err(|e| e.to_string())?;
        camera.open_stream().map_err(|e| e.to_string())?;
        let frame = camera.frame().map_err(|e| e.to_string())?;
        let rgb_image = frame.decode_image::<RgbFormat>().map_err(|e| e.to_string())?;
        let gray_image = image::DynamicImage::ImageRgb8(rgb_image).to_luma8();
        let account = parse_otpauth_uri_from_luma(gray_image).map_err(|e| e.to_string())?;

        self.reload_accounts()?;
        if self.accounts.iter().any(|a| a.label == account.label) {
            return Err("An account with this label already exists.".to_string());
        }
        self.accounts.push(account);
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn generate_current_code(&mut self) -> Result<(), String> {
        self.reload_accounts()?;
        let account = self
            .accounts
            .iter()
            .find(|a| a.label == self.selected_label)
            .ok_or_else(|| "Select an account label first.".to_string())?;
        let code = generate_totp_for_account(account).map_err(|e| e.to_string())?;
        self.generated_code = format_totp_code(code, account.digits);
        Ok(())
    }

    fn export_backup_file(&mut self) -> Result<(), String> {
        if self.backup_passphrase.is_empty() {
            return Err("Backup passphrase is required.".to_string());
        }
        export_backup(
            &self.db_pathbuf(),
            &PathBuf::from(self.backup_path.clone()),
            &self.db_passphrase,
            &self.backup_passphrase,
        )
        .map_err(|e| e.to_string())
    }

    fn import_backup_file(&mut self) -> Result<(), String> {
        if self.backup_passphrase.is_empty() {
            return Err("Backup passphrase is required.".to_string());
        }
        import_backup(
            &PathBuf::from(self.backup_path.clone()),
            &self.db_pathbuf(),
            &self.backup_passphrase,
            &self.db_passphrase,
        )
        .map_err(|e| e.to_string())?;
        self.reload_accounts()?;
        Ok(())
    }

    fn update_selected_account(&mut self) -> Result<(), String> {
        if self.selected_label.is_empty() {
            return Err("Select an account first.".to_string());
        }
        if self.edit_label.trim().is_empty() || self.edit_issuer.trim().is_empty() {
            return Err("Edit issuer and label are required.".to_string());
        }

        let period_seconds = self
            .edit_period
            .trim()
            .parse::<u32>()
            .map_err(|_| "Period must be a positive number (seconds).".to_string())?;
        let digits = self
            .edit_digits
            .trim()
            .parse::<u8>()
            .map_err(|_| "Digits must be a number (typically 6 or 8).".to_string())?;
        validate_period(period_seconds).map_err(|e| e.to_string())?;
        validate_digits(digits).map_err(|e| e.to_string())?;

        self.reload_accounts()?;
        let index = self
            .accounts
            .iter()
            .position(|a| a.label == self.selected_label)
            .ok_or_else(|| "Selected account no longer exists.".to_string())?;

        if self
            .accounts
            .iter()
            .enumerate()
            .any(|(i, a)| i != index && a.label == self.edit_label)
        {
            return Err("Another account already uses this label.".to_string());
        }

        self.accounts[index].issuer = self.edit_issuer.clone();
        self.accounts[index].label = self.edit_label.clone();
        self.accounts[index].algorithm = self.edit_algorithm;
        self.accounts[index].period_seconds = period_seconds;
        self.accounts[index].digits = digits;
        if !self.edit_secret.trim().is_empty() {
            self.accounts[index].secret = decode_secret(&self.edit_secret).map_err(|e| e.to_string())?;
        }
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        self.selected_label = self.edit_label.clone();
        self.reload_accounts()?;
        Ok(())
    }

    fn delete_selected_account(&mut self) -> Result<(), String> {
        if self.selected_label.is_empty() {
            return Err("Select an account first.".to_string());
        }
        self.reload_accounts()?;
        let original_len = self.accounts.len();
        self.accounts.retain(|a| a.label != self.selected_label);
        if self.accounts.len() == original_len {
            return Err("Selected account was not found.".to_string());
        }
        save_accounts(&self.db_pathbuf(), &self.accounts, &self.db_passphrase).map_err(|e| e.to_string())?;
        self.selected_label = self
            .accounts
            .first()
            .map(|a| a.label.clone())
            .unwrap_or_default();
        self.sync_edit_fields_from_selection();
        Ok(())
    }

    fn keychain_entry(&self) -> Result<Entry, String> {
        if self.db_path.trim().is_empty() {
            return Err("Database file path is required.".to_string());
        }
        Ok(Entry::new("custom2fa.desktop", &self.db_path).map_err(|e| e.to_string())?)
    }

    fn save_passphrase_to_keychain(&mut self) -> Result<(), String> {
        if self.db_passphrase.is_empty() {
            return Err("Database passphrase is required.".to_string());
        }
        let entry = self.keychain_entry()?;
        entry
            .set_password(&self.db_passphrase)
            .map_err(|e| e.to_string())
    }

    fn load_passphrase_from_keychain(&mut self) -> Result<(), String> {
        let entry = self.keychain_entry()?;
        self.db_passphrase = entry.get_password().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn clear_passphrase_from_keychain(&mut self) -> Result<(), String> {
        let entry = self.keychain_entry()?;
        entry.delete_credential().map_err(|e| e.to_string())
    }

    fn run_action<F>(&mut self, action: F)
    where
        F: FnOnce(&mut Self) -> Result<(), String>,
    {
        match action(self) {
            Ok(_) => self.status = "Success.".to_string(),
            Err(e) => self.status = format!("Error: {e}"),
        }
    }
}

impl eframe::App for Custom2faApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_defaults();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Custom2FA Hub");
            ui.label("Offline TOTP manager with encrypted local storage.");

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Database file");
                ui.text_edit_singleline(&mut self.db_path);
            });
            ui.horizontal(|ui| {
                ui.label("Database passphrase");
                ui.add(egui::TextEdit::singleline(&mut self.db_passphrase).password(true));
            });
            ui.horizontal(|ui| {
                if ui.button("Save passphrase to OS keychain").clicked() {
                    self.run_action(|s| s.save_passphrase_to_keychain());
                }
                if ui.button("Load passphrase from OS keychain").clicked() {
                    self.run_action(|s| s.load_passphrase_from_keychain());
                }
                if ui.button("Clear keychain entry").clicked() {
                    self.run_action(|s| s.clear_passphrase_from_keychain());
                }
            });
            if ui.button("Load Accounts").clicked() {
                self.run_action(|s| s.reload_accounts());
            }

            ui.separator();
            ui.heading("Add account from manual secret");
            ui.horizontal(|ui| {
                ui.label("Issuer");
                ui.text_edit_singleline(&mut self.issuer);
            });
            ui.horizontal(|ui| {
                ui.label("Label");
                ui.text_edit_singleline(&mut self.label);
            });
            ui.horizontal(|ui| {
                ui.label("Base32 secret");
                ui.text_edit_singleline(&mut self.secret);
            });
            ui.horizontal(|ui| {
                ui.label("Algorithm");
                egui::ComboBox::from_id_source("add_algo")
                    .selected_text(format!("{}", self.add_algorithm))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.add_algorithm, TotpAlgorithm::Sha1, "SHA1");
                        ui.selectable_value(&mut self.add_algorithm, TotpAlgorithm::Sha256, "SHA256");
                        ui.selectable_value(&mut self.add_algorithm, TotpAlgorithm::Sha512, "SHA512");
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Period (seconds)");
                ui.text_edit_singleline(&mut self.add_period);
            });
            ui.horizontal(|ui| {
                ui.label("Digits");
                ui.text_edit_singleline(&mut self.add_digits);
            });
            if ui.button("Add Manual Account").clicked() {
                self.run_action(|s| s.add_manual());
            }

            ui.separator();
            ui.heading("Import account");
            ui.horizontal(|ui| {
                ui.label("OTP URI");
                ui.text_edit_singleline(&mut self.uri);
            });
            if ui.button("Import OTP URI").clicked() {
                self.run_action(|s| s.import_uri());
            }

            ui.horizontal(|ui| {
                ui.label("QR image path");
                ui.text_edit_singleline(&mut self.qr_image_path);
            });
            if ui.button("Import QR Image").clicked() {
                self.run_action(|s| s.import_qr());
            }
            ui.label("Tip: paste path without quotes, or quotes are auto-trimmed.");
            ui.horizontal(|ui| {
                ui.label("Camera index");
                ui.text_edit_singleline(&mut self.camera_index);
                if ui.button("Scan QR From Camera (one frame)").clicked() {
                    self.run_action(|s| s.import_qr_from_camera());
                }
            });

            ui.separator();
            ui.heading("Generate code");
            if self.accounts.is_empty() {
                ui.label("No loaded accounts.");
            } else {
                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.text_edit_singleline(&mut self.search_term);
                });
                let filtered: Vec<(String, String)> = self
                    .filtered_accounts()
                    .into_iter()
                    .map(|a| (a.label.clone(), a.issuer.clone()))
                    .collect();
                egui::ComboBox::from_label("Account label")
                    .selected_text(if self.selected_label.is_empty() {
                        "Select label".to_string()
                    } else {
                        self.selected_label.clone()
                    })
                    .show_ui(ui, |ui| {
                        for (label, issuer) in filtered {
                            ui.selectable_value(
                                &mut self.selected_label,
                                label.clone(),
                                format!("{label} ({issuer})"),
                            );
                        }
                    });
                if ui.button("Load Selected Into Editor").clicked() {
                    self.sync_edit_fields_from_selection();
                }
            }
            if ui.button("Generate Current Code").clicked() {
                self.run_action(|s| s.generate_current_code());
            }
            if ui.button("Copy Code").clicked() {
                if self.generated_code.is_empty() {
                    self.status = "No code to copy. Generate a code first.".to_string();
                } else {
                    ui.ctx().copy_text(self.generated_code.clone());
                    self.status = "Copied current code to clipboard.".to_string();
                }
            }
            if !self.generated_code.is_empty() {
                ui.label(format!("Current code: {}", self.generated_code));
            }

            ui.separator();
            ui.heading("Manage selected account");
            ui.horizontal(|ui| {
                ui.label("Edit issuer");
                ui.text_edit_singleline(&mut self.edit_issuer);
            });
            ui.horizontal(|ui| {
                ui.label("Edit label");
                ui.text_edit_singleline(&mut self.edit_label);
            });
            ui.horizontal(|ui| {
                ui.label("New base32 secret (optional)");
                ui.text_edit_singleline(&mut self.edit_secret);
            });
            ui.horizontal(|ui| {
                ui.label("Algorithm");
                egui::ComboBox::from_id_source("edit_algo")
                    .selected_text(format!("{}", self.edit_algorithm))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.edit_algorithm, TotpAlgorithm::Sha1, "SHA1");
                        ui.selectable_value(&mut self.edit_algorithm, TotpAlgorithm::Sha256, "SHA256");
                        ui.selectable_value(&mut self.edit_algorithm, TotpAlgorithm::Sha512, "SHA512");
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Period (seconds)");
                ui.text_edit_singleline(&mut self.edit_period);
            });
            ui.horizontal(|ui| {
                ui.label("Digits");
                ui.text_edit_singleline(&mut self.edit_digits);
            });
            ui.horizontal(|ui| {
                if ui.button("Update Selected Account").clicked() {
                    self.run_action(|s| s.update_selected_account());
                }
                if ui.button("Delete Selected Account").clicked() {
                    self.run_action(|s| s.delete_selected_account());
                }
            });

            ui.separator();
            ui.heading("Backup");
            ui.horizontal(|ui| {
                ui.label("Backup file");
                ui.text_edit_singleline(&mut self.backup_path);
            });
            ui.horizontal(|ui| {
                ui.label("Backup passphrase");
                ui.add(egui::TextEdit::singleline(&mut self.backup_passphrase).password(true));
            });
            ui.horizontal(|ui| {
                if ui.button("Export Backup").clicked() {
                    self.run_action(|s| s.export_backup_file());
                }
                if ui.button("Import Backup").clicked() {
                    self.run_action(|s| s.import_backup_file());
                }
            });

            ui.separator();
            ui.label(format!("Status: {}", self.status));
        });
    }
}
