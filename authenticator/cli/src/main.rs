use clap::Parser;
use clap::Subcommand;
use custom2fa_core::account::{
    validate_digits, validate_period, Account, TotpAlgorithm,
    DEFAULT_DIGITS, DEFAULT_PERIOD_SECONDS,
};
use custom2fa_core::otp_uri::{
    parse_otpauth_uri, parse_otpauth_uri_from_qr_image, parse_totp_algorithm_label,
};
use custom2fa_core::storage::{export_backup, import_backup, load_accounts, save_accounts};
use custom2fa_core::totp::{decode_secret, format_totp_code, generate_totp_for_account};
use rpassword::prompt_password;
use std::path::PathBuf;
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
    },
    List,
    Code {
        #[arg(long)]
        label: String,
    },
    ImportUri {
        #[arg(long)]
        uri: String,
    },
    ImportQr {
        #[arg(long)]
        image: PathBuf,
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
        } => {
            let secret_bytes = match decode_secret(&secret) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to decode secret: {e}");
                    std::process::exit(1);
                }
            };

            let algorithm_parsed = match algorithm.as_deref() {
                None => TotpAlgorithm::default(),
                Some(s) => match parse_totp_algorithm_label(s) {
                    Ok(a) => a,
                    Err(_) => {
                        eprintln!("Invalid algorithm: use SHA1, SHA256, or SHA512.");
                        std::process::exit(1);
                    }
                },
            };

            let period_seconds = period.unwrap_or(DEFAULT_PERIOD_SECONDS);
            let digits_val = digits.unwrap_or(DEFAULT_DIGITS);
            if let Err(e) = validate_period(period_seconds) {
                eprintln!("Invalid period: {e}");
                std::process::exit(1);
            }
            if let Err(e) = validate_digits(digits_val) {
                eprintln!("Invalid digits: {e}");
                std::process::exit(1);
            }

            let mut accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };

            if accounts.iter().any(|a| a.label == label) {
                eprintln!("An account with this label already exists.");
                std::process::exit(1);
            }

            accounts.push(Account {
                issuer,
                label,
                secret: secret_bytes,
                algorithm: algorithm_parsed,
                period_seconds,
                digits: digits_val,
            });

            if let Err(e) = save_accounts(&args.db, &accounts, &passphrase) {
                eprintln!("Failed to save account database: {e}");
                std::process::exit(1);
            }
            println!("Account added successfully.");
        }
        Command::List => {
            let mut accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };
            accounts.sort_by_key(|a| a.label.to_lowercase());

            if accounts.is_empty() {
                println!("No accounts saved.");
                return;
            }

            for account in accounts {
                println!(
                    "{} ({}) [{} · {}s · {} digits]",
                    account.label,
                    account.issuer,
                    account.algorithm,
                    account.period_seconds,
                    account.digits
                );
            }
        }
        Command::Code { label } => {
            let accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };

            let Some(account) = accounts.into_iter().find(|a| a.label == label) else {
                eprintln!("No account found for label: {label}");
                std::process::exit(1);
            };

            let code = match generate_totp_for_account(&account) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to generate code: {e}");
                    std::process::exit(1);
                }
            };
            println!("{}", format_totp_code(code, account.digits));
        }
        Command::ImportUri { uri } => {
            let account = match parse_otpauth_uri(&uri) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to parse OTP URI: {e}");
                    std::process::exit(1);
                }
            };

            let mut accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };

            if accounts.iter().any(|a| a.label == account.label) {
                eprintln!("An account with this label already exists.");
                std::process::exit(1);
            }

            accounts.push(account);
            if let Err(e) = save_accounts(&args.db, &accounts, &passphrase) {
                eprintln!("Failed to save account database: {e}");
                std::process::exit(1);
            }
            println!("OTP URI imported successfully.");
        }
        Command::ImportQr { image } => {
            let account = match parse_otpauth_uri_from_qr_image(&image) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to import OTP from QR image: {e}");
                    std::process::exit(1);
                }
            };

            let mut accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };

            if accounts.iter().any(|a| a.label == account.label) {
                eprintln!("An account with this label already exists.");
                std::process::exit(1);
            }

            accounts.push(account);
            if let Err(e) = save_accounts(&args.db, &accounts, &passphrase) {
                eprintln!("Failed to save account database: {e}");
                std::process::exit(1);
            }
            println!("QR code imported successfully.");
        }
        Command::ExportBackup {
            backup,
            mut backup_passphrase,
        } => {
            let backup_secret = resolve_passphrase(
                backup_passphrase.take(),
                "Backup passphrase: ",
            );
            let mut backup_secret = match backup_secret {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = export_backup(&args.db, &backup, &passphrase, &backup_secret) {
                eprintln!("Failed to export backup: {e}");
                std::process::exit(1);
            }
            backup_secret.zeroize();
            println!("Backup exported successfully.");
        }
        Command::ImportBackup {
            backup,
            mut backup_passphrase,
        } => {
            let backup_secret = resolve_passphrase(
                backup_passphrase.take(),
                "Backup passphrase: ",
            );
            let mut backup_secret = match backup_secret {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = import_backup(&backup, &args.db, &backup_secret, &passphrase) {
                eprintln!("Failed to import backup: {e}");
                std::process::exit(1);
            }
            backup_secret.zeroize();
            println!("Backup imported and re-encrypted for local database.");
        }
    }

    passphrase.zeroize();
}

fn resolve_passphrase(
    cli_value: Option<String>,
    prompt: &str,
) -> Result<String, &'static str> {
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
