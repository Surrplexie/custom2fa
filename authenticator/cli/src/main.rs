use clap::Parser;
use clap::Subcommand;
use custom2fa_core::account::Account;
use custom2fa_core::storage::{load_accounts, save_accounts};
use custom2fa_core::totp::{current_timestep, decode_secret, generate_totp};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "custom2fa")]
#[command(about = "Offline-first TOTP authenticator CLI")]
struct Args {
    #[arg(short, long, default_value = "accounts.c2fa")]
    db: PathBuf,

    #[arg(short, long)]
    passphrase: String,

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
}

fn main() {
    let args = Args::parse();

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

            let mut accounts = match load_accounts(&args.db, &args.passphrase) {
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

            if let Err(e) = save_accounts(&args.db, &accounts, &args.passphrase) {
                eprintln!("Failed to save account database: {e}");
                std::process::exit(1);
            }
            println!("Account added successfully.");
        }
        Command::List => {
            let accounts = match load_accounts(&args.db, &args.passphrase) {
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
            let accounts = match load_accounts(&args.db, &args.passphrase) {
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
    }
}