use crate::account::Account;
use crate::error::AuthError;
use crate::totp::decode_secret;
use image::ImageReader;
use rqrr::PreparedImage;
use std::path::Path;
use url::Url;

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
    for (k, v) in url.query_pairs() {
        if k == "secret" {
            secret = Some(v.into_owned());
        } else if k == "issuer" {
            issuer_from_query = Some(v.into_owned());
        }
    }

    let issuer = issuer_from_query.or(issuer_from_label).unwrap_or_default();
    let secret = secret.ok_or(AuthError::InvalidOtpUri)?;
    let secret_bytes = decode_secret(&secret)?;

    Ok(Account {
        issuer,
        label,
        secret: secret_bytes,
    })
}

pub fn parse_otpauth_uri_from_qr_image(path: &Path) -> Result<Account, AuthError> {
    let image = ImageReader::open(path)?.decode()?.to_luma8();
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

    #[test]
    fn parses_standard_otpauth_uri() {
        let uri = "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let account = parse_otpauth_uri(uri).expect("uri should parse");
        assert_eq!(account.issuer, "Example");
        assert_eq!(account.label, "alice@example.com");
        assert!(!account.secret.is_empty());
    }

    #[test]
    fn rejects_non_otpauth_scheme() {
        let uri = "https://example.com/not-otp";
        let result = parse_otpauth_uri(uri);
        assert!(result.is_err());
    }
}
