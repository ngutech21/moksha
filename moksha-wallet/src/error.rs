use std::string::FromUtf8Error;

use lightning_invoice::ParseOrSemanticError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MokshaWalletError {
    #[cfg(target_arch = "wasm32")]
    #[error("GlooNetError - {0}")]
    GlooNet(#[from] gloo_net::Error),

    #[error("SerdeJsonError - {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("ReqwestError - {0}")]
    Reqwest(#[from] reqwest::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("InvalidHeaderValueError - {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error("{0}")]
    MintError(String),

    #[error("{1}")]
    InvoiceNotPaidYet(u64, String),

    #[error("UnexpectedResponse - {0}")]
    UnexpectedResponse(String),

    #[error("MokshaCoreError - {0}")]
    MokshaCore(#[from] moksha_core::error::MokshaCoreError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("DB Error {0}")]
    Db(#[from] sqlx::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Migrate Error {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Sqlite Error {0}")]
    Sqlite(#[from] sqlx::sqlite::SqliteError),

    #[error("Utf8 Error {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Invalid Proofs")]
    InvalidProofs,

    #[error("Not enough tokens")]
    NotEnoughTokens,

    #[error("Failed to decode payment request {0} - Error {1}")]
    DecodeInvoice(String, ParseOrSemanticError),

    #[error("Invalid invoice {0}")]
    InvalidInvoice(String),

    #[error("URLParseError - {0}")]
    Url(#[from] url::ParseError),

    #[error("Unsupported version: Only mints with /v1 api are supported")]
    UnsupportedApiVersion,

    #[error("Bip32Error {0}")]
    Bip32(#[from] bip32::Error),

    #[error("Bip39Error {0}")]
    Bip39(#[from] bip39::Error),

    #[error("Secp256k1 {0}")]
    Secp256k1(#[from] secp256k1::Error),

    #[error("Primarykey not set for keyset")]
    IdNotSet,

    #[error("Found multiple seeds in the database. This is not supported.")]
    MultipleSeeds,

    #[error("Not valid hex string")]
    Hex(#[from] hex::FromHexError),

    #[error("Invalid Keyset-ID")]
    Slice(#[from] std::array::TryFromSliceError),

    #[error("Pubkey not found")]
    PubkeyNotFound,
}
