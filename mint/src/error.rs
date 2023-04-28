use std::error::Error;

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

    // FIXME replace DB error with specific errors
    #[error("{0}")]
    Db(String),
    #[error("Invoice amount is too low {0}")]
    InvoiceAmountTooLow(String),
}

impl IntoResponse for CashuMintError {
    fn into_response(self) -> Response {
        let (status, error_message) = (StatusCode::INTERNAL_SERVER_ERROR, self.to_string());

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::CashuMintError;

    #[test]
    fn test_proof() -> anyhow::Result<()> {
        let error = CashuMintError::InvoiceAmountTooLow("test".to_string());
        println!("error: {}", error);

        Ok(())
    }
}
