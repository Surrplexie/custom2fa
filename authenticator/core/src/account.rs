use crate::error::AuthError;
use serde::{Deserialize, Serialize};
use std::fmt;

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
