use hyper::{header::CONTENT_TYPE, http::HeaderValue};
use url::Url;

use crate::model::{CreateInvoiceParams, CreateInvoiceResult, PayInvoiceResult};

#[derive(Debug, thiserror::Error)]
pub enum AlbyError {
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
}

#[derive(Clone)]
pub struct AlbyClient {
    api_key: String,
    alby_url: Url,
    reqwest_client: reqwest::Client,
}

impl AlbyClient {
    pub fn new(api_key: &str) -> Result<AlbyClient, AlbyError> {
        let alby_url = Url::parse("https://api.getalby.com")?;

        let reqwest_client = reqwest::Client::builder().build()?;

        Ok(AlbyClient {
            api_key: api_key.to_owned(),
            alby_url,
            reqwest_client,
        })
    }
}

impl AlbyClient {
    pub async fn make_get(&self, endpoint: &str) -> Result<String, AlbyError> {
        let url = self.alby_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .get(url)
            .bearer_auth(self.api_key.clone())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AlbyError::NotFound);
        }

        Ok(response.text().await?)
    }

    pub async fn make_post(&self, endpoint: &str, body: &str) -> Result<String, AlbyError> {
        let url = self.alby_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .post(url)
            .bearer_auth(self.api_key.clone())
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str("application/json").expect("Invalid header value"),
            )
            .body(body.to_string())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AlbyError::NotFound);
        }

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AlbyError::Unauthorized);
        }

        Ok(response.text().await?)
    }
}

impl AlbyClient {
    pub async fn create_invoice(
        &self,
        params: &CreateInvoiceParams,
    ) -> Result<CreateInvoiceResult, AlbyError> {
        let params = serde_json::json!({
            "amount": params.amount,
            "description": params.memo,
        });

        let body = self
            .make_post("invoices", &serde_json::to_string(&params)?)
            .await?;

        let response: serde_json::Value = serde_json::from_str(&body)?;
        let payment_request = response["payment_request"]
            .as_str()
            .expect("payment_request is empty")
            .to_owned();
        let payment_hash = response["payment_hash"]
            .as_str()
            .expect("payment_hash is empty")
            .to_owned();

        Ok(CreateInvoiceResult {
            payment_hash: payment_hash.as_bytes().to_vec(),
            payment_request,
        })
    }

    pub async fn pay_invoice(&self, bolt11: &str) -> Result<PayInvoiceResult, AlbyError> {
        let body = self
            .make_post(
                "payments/bolt11",
                &serde_json::to_string(&serde_json::json!({"invoice": bolt11 }))?,
            )
            .await?;

        let response: serde_json::Value = serde_json::from_str(&body)?;

        Ok(PayInvoiceResult {
            payment_hash: response["payment_hash"]
                .as_str()
                .expect("payment_hash is empty")
                .to_owned(),
        })
    }

    pub async fn is_invoice_paid(&self, payment_hash: &str) -> Result<bool, AlbyError> {
        let body = self.make_get(&format!("invoices/{payment_hash}")).await?;

        Ok(serde_json::from_str::<serde_json::Value>(&body)?["settled"]
            .as_bool()
            .unwrap_or(false))
    }
}
