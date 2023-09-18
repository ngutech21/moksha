use hyper::{header::CONTENT_TYPE, http::HeaderValue};
use url::Url;

use crate::model::{CreateInvoiceParams, CreateInvoiceResult, PayInvoiceResult};

use super::error::LightningError;

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
    ) -> Result<LNBitsClient, LightningError> {
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
    pub async fn make_get(&self, endpoint: &str) -> Result<String, LightningError> {
        let url = self.lnbits_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .get(url)
            .header("X-Api-Key", self.admin_key.clone())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LightningError::NotFound);
        }

        Ok(response.text().await?)
    }

    pub async fn make_post(&self, endpoint: &str, body: &str) -> Result<String, LightningError> {
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
            return Err(LightningError::NotFound);
        }

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LightningError::Unauthorized);
        }

        Ok(response.text().await?)
    }
}

impl LNBitsClient {
    pub async fn create_invoice(
        &self,
        params: &CreateInvoiceParams,
    ) -> Result<CreateInvoiceResult, LightningError> {
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

    pub async fn pay_invoice(&self, bolt11: &str) -> Result<PayInvoiceResult, LightningError> {
        let body = self
            .make_post(
                "api/v1/payments",
                &serde_json::to_string(&serde_json::json!({ "out": true, "bolt11": bolt11 }))?,
            )
            .await?;

        Ok(serde_json::from_str(&body)?)
    }

    pub async fn is_invoice_paid(&self, payment_hash: &str) -> Result<bool, LightningError> {
        let body = self
            .make_get(&format!("api/v1/payments/{payment_hash}"))
            .await?;

        Ok(serde_json::from_str::<serde_json::Value>(&body)?["paid"]
            .as_bool()
            .unwrap_or(false))
    }
}
