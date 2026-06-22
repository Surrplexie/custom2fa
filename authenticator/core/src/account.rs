use crate::error::AuthError;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroize;

/// Hash algorithm for RFC 6238 TOTP (`otpauth` URI `algorithm` parameter).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TotpAlgorithm {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

impl fmt::Display for TotpAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TotpAlgorithm::Sha1 => write!(f, "SHA1"),
            TotpAlgorithm::Sha256 => write!(f, "SHA256"),
            TotpAlgorithm::Sha512 => write!(f, "SHA512"),
        }
    }
}

pub const DEFAULT_PERIOD_SECONDS: u32 = 30;
pub const DEFAULT_DIGITS: u8 = 6;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub issuer: String,
    pub label: String,
    pub secret: Vec<u8>,
    #[serde(default)]
    pub algorithm: TotpAlgorithm,
    #[serde(default = "default_period_seconds")]
    pub period_seconds: u32,
    #[serde(default = "default_digits")]
    pub digits: u8,
    /// Optional user-defined category / group label.
    /// Stored with the vault; omitted from JSON when empty (backwards-compatible).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
}

fn default_period_seconds() -> u32 {
    DEFAULT_PERIOD_SECONDS
}

fn default_digits() -> u8 {
    DEFAULT_DIGITS
}

impl Account {
    /// Validates `period_seconds` and `digits` for code generation.
    pub fn validate_totp_parameters(&self) -> Result<(), AuthError> {
        validate_period(self.period_seconds)?;
        validate_digits(self.digits)?;
        Ok(())
    }

    /// Scrubs the decoded TOTP secret bytes from memory.
    pub fn zeroize_secrets(&mut self) {
        self.secret.zeroize();
    }
}

/// Scrubs TOTP secret bytes for every account in `accounts`.
pub fn zeroize_accounts(accounts: &mut [Account]) {
    for account in accounts.iter_mut() {
        account.zeroize_secrets();
    }
}

pub fn validate_period(period_seconds: u32) -> Result<(), AuthError> {
    if !(1..=86400).contains(&period_seconds) {
        return Err(AuthError::InvalidTotpParameters);
    }
    Ok(())
}

pub fn validate_digits(digits: u8) -> Result<(), AuthError> {
    if !(4..=10).contains(&digits) {
        return Err(AuthError::InvalidTotpParameters);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{zeroize_accounts, Account, TotpAlgorithm};

    #[test]
    fn zeroize_accounts_scrubs_secret_bytes() {
        let mut accounts = vec![Account {
            issuer: "Test".into(),
            label: "user".into(),
            secret: vec![0xAA, 0xBB, 0xCC],
            algorithm: TotpAlgorithm::Sha1,
            period_seconds: 30,
            digits: 6,
            category: String::new(),
        }];

        zeroize_accounts(&mut accounts);
        assert!(accounts[0].secret.iter().all(|&b| b == 0));
    }
}
