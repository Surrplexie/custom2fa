use clap::Parser;
use core::totp::{generate_totp, current_timestep};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    secret: String,
}

fn main() {
    let args = Args::parse();

    let secret_bytes = base32::decode(
        base32::Alphabet::RFC4648 { padding: false },
        &args.secret,
    )
    .expect("Invalid base32 secret");

    let timestep = current_timestep();
    let code = generate_totp(&secret_bytes, timestep, 6);

    println!("Your TOTP code: {:06}", code);
}