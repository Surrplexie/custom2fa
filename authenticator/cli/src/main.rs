use clap::Parser;
use clap::Subcommand;
use custom2fa_core::account::Account;
use custom2fa_core::otp_uri::{parse_otpauth_uri, parse_otpauth_uri_from_qr_image};
use custom2fa_core::storage::{export_backup, import_backup, load_accounts, save_accounts};
use custom2fa_core::totp::{current_timestep, decode_secret, generate_totp};
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
        } => {
            let secret_bytes = match decode_secret(&secret) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to decode secret: {e}");
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

            if accounts.iter().any(|a| a.label == label) {
                eprintln!("An account with this label already exists.");
                std::process::exit(1);
            }

            accounts.push(Account {
                issuer,
                label,
                secret: secret_bytes,
            });

            if let Err(e) = save_accounts(&args.db, &accounts, &passphrase) {
                eprintln!("Failed to save account database: {e}");
                std::process::exit(1);
            }
            println!("Account added successfully.");
        }
        Command::List => {
            let accounts = match load_accounts(&args.db, &passphrase) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load account database: {e}");
                    std::process::exit(1);
                }
            };

            if accounts.is_empty() {
                println!("No accounts saved.");
                return;
            }

            for account in accounts {
                println!("{} ({})", account.label, account.issuer);
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

            let timestep = current_timestep();
            let code = generate_totp(&account.secret, timestep, 6);
            println!("{:06}", code);
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