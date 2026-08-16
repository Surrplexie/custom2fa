use clap::Parser;
use clap::Subcommand;
use custom2fa_core::account::{
    label_for_secret, validate_digits, validate_period, Account, TotpAlgorithm, DEFAULT_DIGITS,
    DEFAULT_PERIOD_SECONDS,
};
use custom2fa_core::otp_uri::{
    parse_otpauth_uri, parse_otpauth_uri_from_qr_image, parse_totp_algorithm_label,
};
use custom2fa_core::storage::{
    change_passphrase, export_backup, import_backup, load_accounts, save_accounts,
};
use custom2fa_core::totp::{decode_secret, format_totp_code, generate_totp_for_account};
use rpassword::prompt_password;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

#[derive(Parser, Debug)]
#[command(name = "custom2fa")]
#[command(about = "Offline-first TOTP authenticator CLI")]
struct Args {
    #[arg(short, long, default_value = "accounts.c2fa")]
    db: PathBuf,

    #[arg(short, long)]
    passphrase: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Add {
        #[arg(long)]
        issuer: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        secret: String,
        /// TOTP hash: SHA1, SHA256, or SHA512 (default SHA1).
        #[arg(long)]
        algorithm: Option<String>,
        /// TOTP period in seconds (default 30).
        #[arg(long)]
        period: Option<u32>,
        /// Number of digits (default 6).
        #[arg(long)]
        digits: Option<u8>,
        #[arg(long, default_value = "")]
        category: String,
        /// Add even if another account already uses this secret.
        #[arg(long)]
        force: bool,
    },
    Edit {
        #[arg(long)]
        label: String,
        #[arg(long)]
        new_label: Option<String>,
        #[arg(long)]
        issuer: Option<String>,
        #[arg(long)]
        secret: Option<String>,
        #[arg(long)]
        algorithm: Option<String>,
        #[arg(long)]
        period: Option<u32>,
        #[arg(long)]
        digits: Option<u8>,
        #[arg(long)]
        category: Option<String>,
    },
    Delete {
        #[arg(long)]
        label: String,
        /// Required to actually delete.
        #[arg(long)]
        yes: bool,
    },
    List {
        /// Machine-readable JSON array.
        #[arg(long)]
        json: bool,
    },
    Code {
        #[arg(long)]
        label: String,
        /// Refresh the code until interrupted.
        #[arg(long)]
        watch: bool,
    },
    ImportUri {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        force: bool,
    },
    ImportQr {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        force: bool,
    },
    ExportBackup {
        #[arg(long)]
        backup: PathBuf,
        #[arg(long)]
        backup_passphrase: Option<String>,
    },
    ImportBackup {
        #[arg(long)]
        backup: PathBuf,
        #[arg(long)]
        backup_passphrase: Option<String>,
        /// Overwrite the vault instead of merging.
        #[arg(long)]
        replace: bool,
    },
    ChangePassphrase {
        #[arg(long)]
        new_passphrase: Option<String>,
    },
}

fn main() {
    let mut args = Args::parse();
    let mut passphrase = match resolve_passphrase(args.passphrase.take(), "Database passphrase: ") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    match args.command {
        Command::Add {
            issuer,
            label,
            secret,
            algorithm,
            period,
            digits,
            category,
            force,
        } => {
            let secret_bytes = match decode_secret(&secret) {
                Ok(s) => s,
                Err(e) => die(format!("Failed to decode secret: {e}")),
            };

            let algorithm_parsed = parse_algorithm_opt(algorithm.as_deref());
            let period_seconds = period.unwrap_or(DEFAULT_PERIOD_SECONDS);
            let digits_val = digits.unwrap_or(DEFAULT_DIGITS);
            require_totp_params(period_seconds, digits_val);

            let mut accounts = load_db(&args.db, &passphrase);
            if accounts.iter().any(|a| a.label == label) {
                die("An account with this label already exists.");
            }
            if !force {
                if let Some(existing) = label_for_secret(&accounts, &secret_bytes) {
                    die(format!(
                        "This secret is already stored as \"{existing}\". Pass --force to add anyway."
                    ));
                }
            }

            accounts.push(Account {
                issuer,
                label,
                secret: secret_bytes,
                algorithm: algorithm_parsed,
                period_seconds,
                digits: digits_val,
                category,
            });
            save_db(&args.db, &accounts, &passphrase);
            println!("Account added successfully.");
        }
        Command::Edit {
            label,
            new_label,
            issuer,
            secret,
            algorithm,
            period,
            digits,
            category,
        } => {
            let mut accounts = load_db(&args.db, &passphrase);
            let idx = accounts
                .iter()
                .position(|a| a.label == label)
                .unwrap_or_else(|| die(format!("No account found for label: {label}")));

            if let Some(ref nl) = new_label {
                if accounts
                    .iter()
                    .enumerate()
                    .any(|(i, a)| i != idx && a.label == *nl)
                {
                    die("Another account already uses this label.");
                }
                accounts[idx].label = nl.clone();
            }
            if let Some(issuer) = issuer {
                accounts[idx].issuer = issuer;
            }
            if let Some(secret) = secret {
                match decode_secret(&secret) {
                    Ok(bytes) => accounts[idx].secret = bytes,
                    Err(e) => die(format!("Failed to decode secret: {e}")),
                }
            }
            if let Some(algorithm) = algorithm {
                accounts[idx].algorithm = parse_algorithm_opt(Some(&algorithm));
            }
            if let Some(period) = period {
                require_totp_params(period, accounts[idx].digits);
                accounts[idx].period_seconds = period;
            }
            if let Some(digits) = digits {
                require_totp_params(accounts[idx].period_seconds, digits);
                accounts[idx].digits = digits;
            }
            if let Some(category) = category {
                accounts[idx].category = category;
            }
            save_db(&args.db, &accounts, &passphrase);
            println!("Account updated.");
        }
        Command::Delete { label, yes } => {
            if !yes {
                die("Refusing to delete without --yes.");
            }
            let mut accounts = load_db(&args.db, &passphrase);
            let before = accounts.len();
            accounts.retain(|a| a.label != label);
            if accounts.len() == before {
                die(format!("No account found for label: {label}"));
            }
            save_db(&args.db, &accounts, &passphrase);
            println!("Account deleted.");
        }
        Command::List { json } => {
            let mut accounts = load_db(&args.db, &passphrase);
            accounts.sort_by_key(|a| a.label.to_lowercase());

            if accounts.is_empty() {
                if json {
                    println!("[]");
                } else {
                    println!("No accounts saved.");
                }
                passphrase.zeroize();
                return;
            }

            if json {
                let rows: Vec<serde_json::Value> = accounts
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "label": a.label,
                            "issuer": a.issuer,
                            "category": a.category,
                            "algorithm": a.algorithm.to_string(),
                            "period_seconds": a.period_seconds,
                            "digits": a.digits,
                        })
                    })
                    .collect();
                match serde_json::to_string_pretty(&rows) {
                    Ok(s) => println!("{s}"),
                    Err(e) => die(format!("Failed to serialize list: {e}")),
                }
            } else {
                for account in accounts {
                    let cat = if account.category.is_empty() {
                        String::new()
                    } else {
                        format!(" {{{}}}", account.category)
                    };
                    println!(
                        "{} ({}){cat} [{} · {}s · {} digits]",
                        account.label,
                        account.issuer,
                        account.algorithm,
                        account.period_seconds,
                        account.digits
                    );
                }
            }
        }
        Command::Code { label, watch } => {
            let accounts = load_db(&args.db, &passphrase);
            let Some(account) = accounts.into_iter().find(|a| a.label == label) else {
                die(format!("No account found for label: {label}"));
            };

            loop {
                let code = match generate_totp_for_account(&account) {
                    Ok(c) => format_totp_code(c, account.digits),
                    Err(e) => die(format!("Failed to generate code: {e}")),
                };
                let secs = remaining_secs(account.period_seconds);
                if watch {
                    print!("{code}  {secs}s remaining    \r");
                    let _ = io::stdout().flush();
                    thread::sleep(Duration::from_millis(250));
                } else {
                    println!("{code}");
                    break;
                }
            }
        }
        Command::ImportUri { uri, force } => {
            let account = match parse_otpauth_uri(&uri) {
                Ok(a) => a,
                Err(e) => die(format!("Failed to parse OTP URI: {e}")),
            };
            push_imported(&args.db, &passphrase, account, force);
            println!("OTP URI imported successfully.");
        }
        Command::ImportQr { image, force } => {
            let account = match parse_otpauth_uri_from_qr_image(&image) {
                Ok(a) => a,
                Err(e) => die(format!("Failed to import OTP from QR image: {e}")),
            };
            push_imported(&args.db, &passphrase, account, force);
            println!("QR code imported successfully.");
        }
        Command::ExportBackup {
            backup,
            mut backup_passphrase,
        } => {
            let mut backup_secret =
                match resolve_passphrase(backup_passphrase.take(), "Backup passphrase: ") {
                    Ok(p) => p,
                    Err(e) => die(e),
                };
            if let Err(e) = export_backup(&args.db, &backup, &passphrase, &backup_secret) {
                die(format!("Failed to export backup: {e}"));
            }
            backup_secret.zeroize();
            println!("Backup exported successfully.");
        }
        Command::ImportBackup {
            backup,
            mut backup_passphrase,
            replace,
        } => {
            let mut backup_secret =
                match resolve_passphrase(backup_passphrase.take(), "Backup passphrase: ") {
                    Ok(p) => p,
                    Err(e) => die(e),
                };
            match import_backup(&backup, &args.db, &backup_secret, &passphrase, replace) {
                Ok(result) => {
                    backup_secret.zeroize();
                    if result.replaced {
                        println!(
                            "Backup imported (replaced vault with {} account(s)).",
                            result.added
                        );
                    } else {
                        println!(
                            "Backup merged: {} added, {} skipped (duplicate labels).",
                            result.added,
                            result.skipped_labels.len()
                        );
                        for (incoming, existing) in result.duplicate_secrets {
                            eprintln!(
                                "warning: \"{incoming}\" uses the same secret as \"{existing}\""
                            );
                        }
                    }
                }
                Err(e) => die(format!("Failed to import backup: {e}")),
            }
        }
        Command::ChangePassphrase { new_passphrase } => {
            let mut new_pass = match resolve_passphrase(new_passphrase, "New database passphrase: ")
            {
                Ok(p) => p,
                Err(e) => die(e),
            };
            if let Err(e) = change_passphrase(&args.db, &passphrase, &new_pass) {
                die(format!("Failed to change passphrase: {e}"));
            }
            new_pass.zeroize();
            println!("Vault passphrase changed.");
        }
    }

    passphrase.zeroize();
}

fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn load_db(path: &Path, passphrase: &str) -> Vec<Account> {
    match load_accounts(path, passphrase) {
        Ok(a) => a,
        Err(e) => die(format!("Failed to load account database: {e}")),
    }
}

fn save_db(path: &Path, accounts: &[Account], passphrase: &str) {
    if let Err(e) = save_accounts(path, accounts, passphrase) {
        die(format!("Failed to save account database: {e}"));
    }
}

fn parse_algorithm_opt(raw: Option<&str>) -> TotpAlgorithm {
    match raw {
        None => TotpAlgorithm::default(),
        Some(s) => match parse_totp_algorithm_label(s) {
            Ok(a) => a,
            Err(_) => die("Invalid algorithm: use SHA1, SHA256, or SHA512."),
        },
    }
}

fn require_totp_params(period: u32, digits: u8) {
    if let Err(e) = validate_period(period) {
        die(format!("Invalid period: {e}"));
    }
    if let Err(e) = validate_digits(digits) {
        die(format!("Invalid digits: {e}"));
    }
}

fn remaining_secs(period: u32) -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let p = u64::from(period.max(1));
    (p - (now % p)) as u32
}

fn push_imported(db: &Path, passphrase: &str, account: Account, force: bool) {
    let mut accounts = load_db(db, passphrase);
    if accounts.iter().any(|a| a.label == account.label) {
        die("An account with this label already exists.");
    }
    if !force {
        if let Some(existing) = label_for_secret(&accounts, &account.secret) {
            die(format!(
                "This secret is already stored as \"{existing}\". Pass --force to add anyway."
            ));
        }
    }
    accounts.push(account);
    save_db(db, &accounts, passphrase);
}

fn resolve_passphrase(cli_value: Option<String>, prompt: &str) -> Result<String, &'static str> {
    match cli_value {
        Some(value) => {
            if value.is_empty() {
                Err("Passphrase cannot be empty.")
            } else {
                Ok(value)
            }
        }
        None => {
            let value =
                prompt_password(prompt).map_err(|_| "Failed to read passphrase from terminal.")?;
            if value.is_empty() {
                Err("Passphrase cannot be empty.")
            } else {
                Ok(value)
            }
        }
    }
}
