use hyper::{header::CONTENT_TYPE, http::HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum LNBitsError {
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
pub struct LNBitsClient {
    admin_key: String,
    lnbits_url: Url,
    reqwest_client: reqwest::Client,
}

impl LNBitsClient {
    pub fn new(
        admin_key: &str,
        lnbits_url: &str,
        tor_socket: Option<&str>,
    ) -> Result<LNBitsClient, LNBitsError> {
        let lnbits_url = Url::parse(lnbits_url)?;

        let reqwest_client = {
            if let Some(tor_socket) = tor_socket {
                let proxy = reqwest::Proxy::all(tor_socket).expect("tor proxy should be there");
                reqwest::Client::builder().proxy(proxy).build()?
            } else {
                reqwest::Client::builder().build()?
            }
        };

        Ok(LNBitsClient {
            admin_key: admin_key.to_string(),
            lnbits_url,
            reqwest_client,
        })
    }
}

impl LNBitsClient {
    pub async fn make_get(&self, endpoint: &str) -> Result<String, LNBitsError> {
        let url = self.lnbits_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .get(url)
            .header("X-Api-Key", self.admin_key.clone())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LNBitsError::NotFound);
        }

        Ok(response.text().await?)
    }

    pub async fn make_post(&self, endpoint: &str, body: &str) -> Result<String, LNBitsError> {
        let url = self.lnbits_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .post(url)
            .header("X-Api-Key", self.admin_key.clone())
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str("application/json").expect("Invalid header value"),
            )
            .body(body.to_string())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LNBitsError::NotFound);
        }

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LNBitsError::Unauthorized);
        }

        Ok(response.text().await?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceResult {
    pub payment_hash: Vec<u8>,
    pub payment_request: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayInvoiceResult {
    pub payment_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceParams {
    pub amount: u64,
    pub unit: String,
    pub memo: Option<String>,
    pub expiry: Option<u32>,
    pub webhook: Option<String>,
    pub internal: Option<bool>,
}

impl LNBitsClient {
    pub async fn create_invoice(
        &self,
        params: &CreateInvoiceParams,
    ) -> Result<CreateInvoiceResult, LNBitsError> {
        // Add out: true to the params
        let params = serde_json::json!({
            "out": false,
            "amount": params.amount,
            "unit": params.unit,
            "memo": params.memo,
            "webhook": params.webhook,
            "internal": params.internal,
            "expiry": params.expiry,
        });

        let body = self
            .make_post("api/v1/payments", &serde_json::to_string(&params)?)
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

    pub async fn pay_invoice(&self, bolt11: &str) -> Result<PayInvoiceResult, LNBitsError> {
        let body = self
            .make_post(
                "api/v1/payments",
                &serde_json::to_string(&serde_json::json!({ "out": true, "bolt11": bolt11 }))?,
            )
            .await?;

        Ok(serde_json::from_str(&body)?)
    }

    pub async fn is_invoice_paid(&self, payment_hash: &str) -> Result<bool, LNBitsError> {
        let body = self
            .make_get(&format!("api/v1/payments/{payment_hash}"))
            .await?;

        Ok(serde_json::from_str::<serde_json::Value>(&body)?["paid"]
            .as_bool()
            .unwrap_or(false))
    }
}
