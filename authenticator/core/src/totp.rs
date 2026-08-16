use crate::account::{validate_digits, validate_period, Account, TotpAlgorithm};
use crate::error::AuthError;
use base32::{decode, Alphabet};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use time::OffsetDateTime;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;

/// Strips whitespace, hyphens, and Base32 padding so pasted secrets still decode.
pub fn normalize_secret_str(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        .collect::<String>()
        .to_ascii_uppercase()
}

/// Converts a Base32 string (from Google/Amazon) into raw bytes.
pub fn decode_secret(base32_secret: &str) -> Result<Vec<u8>, AuthError> {
    let normalized = normalize_secret_str(base32_secret);
    if normalized.is_empty() {
        return Err(AuthError::InvalidSecret);
    }
    decode(Alphabet::RFC4648 { padding: false }, &normalized).ok_or(AuthError::InvalidSecret)
}

/// Unix timestep counter: floor(epoch_seconds / period_seconds).
pub fn timestep_at(unix_seconds: i64, period_seconds: u32) -> Result<u64, AuthError> {
    validate_period(period_seconds)?;
    let p = i64::from(period_seconds);
    Ok((unix_seconds.div_euclid(p)) as u64)
}

/// Current timestep for the given period (UTC).
pub fn current_timestep_for_period(period_seconds: u32) -> Result<u64, AuthError> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    timestep_at(now, period_seconds)
}

/// Current timestep for the default 30-second period (UTC).
pub fn current_timestep() -> u64 {
    OffsetDateTime::now_utc().unix_timestamp().div_euclid(30) as u64
}

fn dynamic_truncation(hmac_result: &[u8]) -> u32 {
    let offset = (hmac_result[hmac_result.len() - 1] & 0x0f) as usize;
    ((u32::from(hmac_result[offset]) & 0x7f) << 24)
        | (u32::from(hmac_result[offset + 1]) << 16)
        | (u32::from(hmac_result[offset + 2]) << 8)
        | u32::from(hmac_result[offset + 3])
}

fn hmac_otp(secret: &[u8], timestep: u64, algorithm: TotpAlgorithm) -> Result<Vec<u8>, AuthError> {
    let counter = timestep.to_be_bytes();
    match algorithm {
        TotpAlgorithm::Sha1 => {
            let mut mac = HmacSha1::new_from_slice(secret).map_err(|_| AuthError::InvalidSecret)?;
            mac.update(&counter);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        TotpAlgorithm::Sha256 => {
            let mut mac =
                HmacSha256::new_from_slice(secret).map_err(|_| AuthError::InvalidSecret)?;
            mac.update(&counter);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        TotpAlgorithm::Sha512 => {
            let mut mac =
                HmacSha512::new_from_slice(secret).map_err(|_| AuthError::InvalidSecret)?;
            mac.update(&counter);
            Ok(mac.finalize().into_bytes().to_vec())
        }
    }
}

/// Generates the current TOTP for a stored account (UTC clock).
pub fn generate_totp_for_account(account: &Account) -> Result<u32, AuthError> {
    account.validate_totp_parameters()?;
    let ts = current_timestep_for_period(account.period_seconds)?;
    generate_totp(&account.secret, ts, account.digits, account.algorithm)
}

/// Format a numeric TOTP code with fixed width (leading zeros).
pub fn format_totp_code(code: u32, digits: u8) -> String {
    format!("{:01$}", code, usize::from(digits))
}

/// RFC 6238 TOTP using the given hash algorithm and digit count.
pub fn generate_totp(
    secret: &[u8],
    timestep: u64,
    digits: u8,
    algorithm: TotpAlgorithm,
) -> Result<u32, AuthError> {
    validate_digits(digits)?;
    let bytes = hmac_otp(secret, timestep, algorithm)?;
    let code = dynamic_truncation(&bytes);
    Ok(code % 10u32.pow(u32::from(digits)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::TotpAlgorithm;

    /// RFC 6238 Appendix B — secret is the ASCII string `12345678901234567890`.
    fn rfc_secret_bytes() -> Vec<u8> {
        b"12345678901234567890".to_vec()
    }

    #[test]
    fn rfc_sha1_t59_8_digits() {
        let ts = timestep_at(59, 30).unwrap();
        assert_eq!(ts, 1);
        let code = generate_totp(&rfc_secret_bytes(), ts, 8, TotpAlgorithm::Sha1).unwrap();
        assert_eq!(code, 94287082);
    }

    #[test]
    fn rfc_sha256_t59_8_digits() {
        let ts = timestep_at(59, 30).unwrap();
        let code = generate_totp(&rfc_secret_bytes(), ts, 8, TotpAlgorithm::Sha256).unwrap();
        assert_eq!(code, 32247374);
    }

    #[test]
    fn rfc_sha512_t59_8_digits() {
        let ts = timestep_at(59, 30).unwrap();
        let code = generate_totp(&rfc_secret_bytes(), ts, 8, TotpAlgorithm::Sha512).unwrap();
        assert_eq!(code, 69342147);
    }

    #[test]
    fn invalid_digits_rejected() {
        let err = generate_totp(&rfc_secret_bytes(), 1, 3, TotpAlgorithm::Sha1);
        assert!(matches!(err, Err(AuthError::InvalidTotpParameters)));
    }

    #[test]
    fn decode_secret_strips_spaces_hyphens_and_padding() {
        let compact = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        let spaced = decode_secret("jbsw y3dp ehpk 3pxp").unwrap();
        let hyphenated = decode_secret("JBSW-Y3DP-EHPK-3PXP").unwrap();
        let padded = decode_secret("JBSWY3DPEHPK3PXP=").unwrap();
        assert_eq!(compact, spaced);
        assert_eq!(compact, hyphenated);
        assert_eq!(compact, padded);
    }

    #[test]
    fn decode_secret_rejects_empty_after_normalize() {
        assert!(matches!(
            decode_secret(" - = \t"),
            Err(AuthError::InvalidSecret)
        ));
    }
}
