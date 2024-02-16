//! This module defines the `MokshaCoreError` enum, which represents the possible errors that can occur in the Moksha Core library.
//!
//! The `MokshaCoreError` enum is derived from the `Error` trait using the `thiserror` crate, which allows for easy definition of custom error types with automatic conversion to and from other error types.
//! All of the variants in the `MokshaCoreError` enum implement the `Error` trait, which allows them to be used with the `?` operator for easy error propagation. The enum is also serializable and deserializable using serde.

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

    #[error("Invalid token")]
    InvalidToken,
}
