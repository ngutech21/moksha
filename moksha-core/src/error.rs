use base64::DecodeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MokshaCoreError {
    #[error("Secp256k1Error {0}")]
    Secp256k1Error(#[from] secp256k1::Error),

    #[error("InvalidTokenType")]
    InvalidTokenPrefix,

    #[error("Base64DecodeError {0}")]
    Base64DecodeError(#[from] DecodeError),

    #[error("SerdeJsonError {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Invalid Keysetid")]
    InvalidKeysetid,

    #[error("Not enough tokens")]
    NotEnoughTokens,
}
