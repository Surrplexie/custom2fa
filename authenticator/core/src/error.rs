use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid secret")]
    InvalidSecret,

    #[error("Encryption error")]
    EncryptionError,
}