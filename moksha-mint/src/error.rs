use std::string::FromUtf8Error;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use lightning_invoice::ParseOrSemanticError;
use serde_json::json;
use thiserror::Error;
use tonic_lnd::ConnectError;
use tracing::{event, Level};

use crate::lightning::error::LightningError;

#[derive(Error, Debug)]
pub enum MokshaMintError {
    #[error("LndConnectError - {0}")]
    ConnectError(ConnectError),

    #[error("Failed to decode payment request {0} - Error {1}")]
    DecodeInvoice(String, ParseOrSemanticError),

    #[error("Failed to pay invoice {0} - Error {1}")]
    PayInvoice(String, LightningError),

    #[error("DB Error {0}")]
    Db(#[from] rocksdb::Error),

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

    #[error("Proof already used {0}")]
    ProofAlreadyUsed(String),

    #[error("{0}")]
    SplitAmountMismatch(String),

    #[error("split amount is higher than the total sum.")]
    SplitAmountTooHigh,

    #[error("duplicate promises.")]
    SplitHasDuplicatePromises,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Lightning Error {0}")]
    Lightning(#[from] LightningError),
}

impl IntoResponse for MokshaMintError {
    fn into_response(self) -> Response {
        event!(Level::ERROR, "error in mint: {:?}", self);

        let status = match self {
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvoiceNotPaidYet => StatusCode::OK,
            _ => StatusCode::BAD_REQUEST,
        };

        let body = Json(json!({
            "code": 0,
            "error": self.to_string(),
        }));

        (status, body).into_response()
    }
}
