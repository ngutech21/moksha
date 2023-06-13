use serde::{Deserialize, Serialize};
use url::Url;

pub enum LNBitsRequestKey {
    Admin,
    InvoiceRead,
}

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
    // wallet_id: String, // FIXME Can be used later
    admin_key: String,
    invoice_read_key: String,
    lnbits_url: Url,
    // tor_socket: Option<String>, // Can be used later
    reqwest_client: reqwest::Client,
}

impl LNBitsClient {
    pub fn new(
        // FIXME Can be used later
        _wallet_id: &str,
        admin_key: &str,
        invoice_read_key: &str,
        lnbits_url: &str,
        tor_socket: Option<&str>,
    ) -> Result<LNBitsClient, LNBitsError> {
        let lnbits_url = Url::parse(lnbits_url)?;

        let client = {
            if let Some(tor_socket) = tor_socket {
                let proxy = reqwest::Proxy::all(tor_socket).expect("tor proxy should be there");
                reqwest::Client::builder().proxy(proxy).build()?
            } else {
                reqwest::Client::builder().build()?
            }
        };

        Ok(LNBitsClient {
            admin_key: admin_key.to_string(),
            invoice_read_key: invoice_read_key.to_string(),
            lnbits_url,
            reqwest_client: client,
        })
    }
}

impl LNBitsClient {
    pub async fn make_get(
        &self,
        endpoint: &str,
        key: LNBitsRequestKey,
    ) -> Result<String, LNBitsError> {
        let url = self.lnbits_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .get(url)
            .header("X-Api-Key", {
                match key {
                    LNBitsRequestKey::Admin => self.admin_key.clone(),
                    LNBitsRequestKey::InvoiceRead => self.invoice_read_key.clone(),
                }
            })
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LNBitsError::NotFound);
        }

        let body = response.text().await?;

        Ok(body)
    }

    pub async fn make_post(
        &self,
        endpoint: &str,
        key: LNBitsRequestKey,
        body: &str,
    ) -> Result<String, LNBitsError> {
        let url = self.lnbits_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .post(url)
            .header("X-Api-Key", {
                match key {
                    LNBitsRequestKey::Admin => self.admin_key.clone(),
                    LNBitsRequestKey::InvoiceRead => self.invoice_read_key.clone(),
                }
            })
            .body(body.to_string())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LNBitsError::NotFound);
        }

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LNBitsError::Unauthorized);
        }

        let body = response.text().await?;

        Ok(body)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceResult {
    pub payment_hash: String,
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
            .make_post(
                "api/v1/payments",
                LNBitsRequestKey::InvoiceRead,
                &serde_json::to_string(&params)?,
            )
            .await?;

        let invoice_result: CreateInvoiceResult = serde_json::from_str(&body)?;
        Ok(invoice_result)
    }

    pub async fn pay_invoice(&self, bolt11: &str) -> Result<PayInvoiceResult, LNBitsError> {
        let body = self
            .make_post(
                "api/v1/payments",
                LNBitsRequestKey::Admin,
                &serde_json::to_string(&serde_json::json!({ "out": true, "bolt11": bolt11 }))?,
            )
            .await?;

        let invoice_result: PayInvoiceResult = serde_json::from_str(&body)?;
        Ok(invoice_result)
    }

    pub async fn is_invoice_paid(&self, payment_hash: &str) -> Result<bool, LNBitsError> {
        let body = self
            .make_get(
                &format!("api/v1/payments/{payment_hash}"),
                LNBitsRequestKey::Admin,
            )
            .await?;

        let invoice_result: serde_json::Value = serde_json::from_str(&body)?;
        Ok(invoice_result["paid"].as_bool().unwrap_or(false))
    }
}
