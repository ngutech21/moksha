use std::string::FromUtf8Error;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use fedimint_tonic_lnd::{tonic::Status, ConnectError};

use lightning_invoice::ParseOrSemanticError;
use moksha_core::primitives::CurrencyUnit;
use serde_json::json;
use thiserror::Error;
use tracing::{event, Level};

use crate::lightning::error::LightningError;

#[derive(Error, Debug)]
pub enum MokshaMintError {
    #[error("LndConnectError - {0}")]
    ConnectError(ConnectError),

    #[error("ClnConnectError - {0}")]
    ClnConnectError(anyhow::Error),

    #[error("Failed to decode payment request {0} - Error {1}")]
    DecodeInvoice(String, ParseOrSemanticError),

    #[error("Failed to pay invoice {0} - Error {1}")]
    PayInvoice(String, LightningError),

    #[error("DB Error {0}")]
    Db(#[from] sqlx::Error),

    #[error("Utf8 Error {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Serde Error {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invoice amount is too low {0}")]
    InvoiceAmountTooLow(String),

    #[error("Invoice not found for hash {0}")]
    InvoiceNotFound(String),

    #[error("Lightning invoice not paid yet.")]
    InvoiceNotPaidYet,

    #[error("BTC-Onchain not paid yet.")]
    BtcOnchainNotPaidYet,

    #[error("Proof already used {0}")]
    ProofAlreadyUsed(String),

    #[error("{0}")]
    SwapAmountMismatch(String),

    #[error("duplicate promises.")]
    SwapHasDuplicatePromises,

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Lightning Error {0}")]
    Lightning(#[from] LightningError),

    #[error("Invalid quote {0}")]
    InvalidQuote(String),

    #[error("Invalid quote uuid {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("Keyset not found {0}")]
    KeysetNotFound(String),

    #[error("Currency not supported {0}")]
    CurrencyNotSupported(CurrencyUnit),

    #[error("Not Enough tokens: {0}")]
    NotEnoughTokens(String),

    #[error("Lnd error: {0}")]
    Lnd(#[from] Status),
}

impl IntoResponse for MokshaMintError {
    fn into_response(self) -> Response {
        event!(Level::ERROR, "error in mint: {:?}", self);

        let body = Json(json!({
            "code": 0,
            "detail": self.to_string(),
        }));

        (StatusCode::BAD_REQUEST, body).into_response()
    }
}
