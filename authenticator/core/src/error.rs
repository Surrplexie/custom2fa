use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid secret")]
    InvalidSecret,

    #[error("Invalid OTP URI")]
    InvalidOtpUri,

    #[error("QR code decode error")]
    QrDecodeError,

    #[error("No QR code found in image")]
    NoQrCodeFound,

    #[error("Encryption error")]
    EncryptionError,

    #[error("Decryption error")]
    DecryptionError,

    #[error("Invalid passphrase")]
    InvalidPassphrase,

    #[error("Invalid ciphertext format")]
    InvalidCiphertext,

    #[error("Unsupported ciphertext version")]
    UnsupportedVersion,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Image decode error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}