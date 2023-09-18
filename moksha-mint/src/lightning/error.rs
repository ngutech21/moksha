#[derive(Debug, thiserror::Error)]
pub enum LightningError {
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("url error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Not found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Payment failed")]
    PaymentFailed,
}
