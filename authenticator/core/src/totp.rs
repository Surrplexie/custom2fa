use crate::error::AuthError;
use base32::{decode, Alphabet};

/// Converts a Base32 string (from Google/Amazon) into raw bytes.
pub fn decode_secret(base32_secret: &str) -> Result<Vec<u8>, AuthError> {
    decode(Alphabet::RFC4648 { padding: false }, &base32_secret.to_uppercase())
        .ok_or(AuthError::InvalidSecret)
}
use hmac::{Hmac, Mac};
use sha1::Sha1;
use time::OffsetDateTime;

type HmacSha1 = Hmac<Sha1>;

pub fn generate_totp(secret: &[u8], timestep: u64, digits: u32) -> u32 {
    let mut mac = HmacSha1::new_from_slice(secret).unwrap();
    mac.update(&timestep.to_be_bytes());

    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0xf) as usize;

    let code = ((u32::from(result[offset]) & 0x7f) << 24)
        | (u32::from(result[offset + 1]) << 16)
        | (u32::from(result[offset + 2]) << 8)
        | (u32::from(result[offset + 3]));

    code % 10_u32.pow(digits)
}


pub fn current_timestep() -> u64 {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    (now / 30) as u64
}