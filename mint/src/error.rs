use std::string::FromUtf8Error;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashuMintError {
    #[error("Failed to decode payment request {0} - Error {1}")]
    DecodeInvoice(String, lnbits_rust::LNBitsError),

    #[error("Failed to pay invoice {0} - Error {1}")]
    PayInvoice(String, lnbits_rust::LNBitsError),

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
}

impl IntoResponse for CashuMintError {
    fn into_response(self) -> Response {
        let status = match self {
            CashuMintError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };

        let body = Json(json!({
            "error": self.to_string(),
        }));

        (status, body).into_response()
    }
}
