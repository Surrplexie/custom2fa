use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid secret")]
    InvalidSecret,

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
}