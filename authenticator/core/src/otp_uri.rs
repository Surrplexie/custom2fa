use crate::account::{validate_digits, validate_period, Account, TotpAlgorithm};
use crate::error::AuthError;
use crate::totp::decode_secret;
use image::GrayImage;
use image::ImageReader;
use rqrr::PreparedImage;
use std::path::Path;
use url::Url;

fn parse_algorithm_param(raw: &str) -> Result<TotpAlgorithm, AuthError> {
    if raw.is_empty() {
        return Ok(TotpAlgorithm::default());
    }
    match raw.to_ascii_uppercase().as_str() {
        "SHA1" => Ok(TotpAlgorithm::Sha1),
        "SHA256" => Ok(TotpAlgorithm::Sha256),
        "SHA512" => Ok(TotpAlgorithm::Sha512),
        _ => Err(AuthError::InvalidOtpUri),
    }
}

pub fn parse_totp_algorithm_label(raw: &str) -> Result<TotpAlgorithm, AuthError> {
    parse_algorithm_param(raw)
}

pub fn parse_otpauth_uri(uri: &str) -> Result<Account, AuthError> {
    let url = Url::parse(uri)?;
    if url.scheme() != "otpauth" {
        return Err(AuthError::InvalidOtpUri);
    }
    if url.host_str() != Some("totp") {
        return Err(AuthError::InvalidOtpUri);
    }

    let path = url.path().trim_start_matches('/');
    if path.is_empty() {
        return Err(AuthError::InvalidOtpUri);
    }

    let mut issuer_from_label = None::<String>;
    let label = if let Some((issuer_part, label_part)) = path.split_once(':') {
        issuer_from_label = Some(issuer_part.to_string());
        label_part.to_string()
    } else {
        path.to_string()
    };

    let mut issuer_from_query = None::<String>;
    let mut secret = None::<String>;
    let mut algorithm_raw = None::<String>;
    let mut period_raw = None::<String>;
    let mut digits_raw = None::<String>;

    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "secret" => secret = Some(v.into_owned()),
            "issuer" => issuer_from_query = Some(v.into_owned()),
            "algorithm" => algorithm_raw = Some(v.into_owned()),
            "period" => period_raw = Some(v.into_owned()),
            "digits" => digits_raw = Some(v.into_owned()),
            _ => {}
        }
    }

    let issuer = issuer_from_query.or(issuer_from_label).unwrap_or_default();
    let secret = secret.ok_or(AuthError::InvalidOtpUri)?;
    let secret_bytes = decode_secret(&secret)?;

    let algorithm = parse_algorithm_param(algorithm_raw.as_deref().unwrap_or(""))?;

    let period_seconds = if let Some(ref p) = period_raw {
        let p = p.parse::<u32>().map_err(|_| AuthError::InvalidOtpUri)?;
        validate_period(p)?;
        p
    } else {
        crate::account::DEFAULT_PERIOD_SECONDS
    };

    let digits = if let Some(ref d) = digits_raw {
        let d = d.parse::<u8>().map_err(|_| AuthError::InvalidOtpUri)?;
        validate_digits(d)?;
        d
    } else {
        crate::account::DEFAULT_DIGITS
    };

    Ok(Account {
        issuer,
        label,
        secret: secret_bytes,
        algorithm,
        period_seconds,
        digits,
    })
}

pub fn parse_otpauth_uri_from_qr_image(path: &Path) -> Result<Account, AuthError> {
    let image = ImageReader::open(path)?.decode()?.to_luma8();
    parse_otpauth_uri_from_luma(image)
}

pub fn parse_otpauth_uri_from_luma(image: GrayImage) -> Result<Account, AuthError> {
    let mut prepared = PreparedImage::prepare(image);
    let grids = prepared.detect_grids();

    for grid in grids {
        let (_, content) = grid.decode().map_err(|_| AuthError::QrDecodeError)?;
        return parse_otpauth_uri(&content);
    }

    Err(AuthError::NoQrCodeFound)
}

#[cfg(test)]
mod tests {
    use super::parse_otpauth_uri;
    use crate::account::TotpAlgorithm;

    #[test]
    fn parses_standard_otpauth_uri() {
        let uri = "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let account = parse_otpauth_uri(uri).expect("uri should parse");
        assert_eq!(account.issuer, "Example");
        assert_eq!(account.label, "alice@example.com");
        assert!(!account.secret.is_empty());
        assert_eq!(account.algorithm, TotpAlgorithm::Sha1);
        assert_eq!(account.period_seconds, 30);
        assert_eq!(account.digits, 6);
    }

    #[test]
    fn parses_algorithm_period_digits() {
        let uri = "otpauth://totp/Test:foo@bar?secret=JBSWY3DPEHPK3PXP&issuer=Test&algorithm=SHA256&period=60&digits=8";
        let account = parse_otpauth_uri(uri).expect("uri should parse");
        assert_eq!(account.algorithm, TotpAlgorithm::Sha256);
        assert_eq!(account.period_seconds, 60);
        assert_eq!(account.digits, 8);
    }

    #[test]
    fn rejects_non_otpauth_scheme() {
        let uri = "https://example.com/not-otp";
        let result = parse_otpauth_uri(uri);
        assert!(result.is_err());
    }
}
