use axum::{
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashuMintError {
    #[error("LnbitsError {0}")]
    Lnbits(#[from] lnbits_rust::LNBitsError),

    #[error("{0}")]
    Db(String),

    #[error("{0}")]
    InvoiceAmountTooLow(String),
}

impl IntoResponse for CashuMintError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            CashuMintError::Lnbits(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            CashuMintError::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            CashuMintError::InvoiceAmountTooLow(e) => (StatusCode::BAD_REQUEST, e),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
