use std::fmt::{self, Formatter};

use async_trait::async_trait;
use clap::Parser;
use hyper::{header::CONTENT_TYPE, http::HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceParams, CreateInvoiceResult, PayInvoiceResult},
};

use super::{error::LightningError, Lightning};

#[derive(Deserialize, Serialize, Debug, Clone, Default, Parser)]
pub struct LnbitsLightningSettings {
    #[clap(long, env = "MINT_LNBITS_ADMIN_KEY")]
    pub admin_key: Option<String>,
    #[clap(long, env = "MINT_LNBITS_URL")]
    pub url: Option<String>, // FIXME use Url type instead
}

impl LnbitsLightningSettings {
    pub fn new(admin_key: &str, url: &str) -> Self {
        Self {
            admin_key: Some(admin_key.to_owned()),
            url: Some(url.to_owned()),
        }
    }
}

impl fmt::Display for LnbitsLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "admin_key: {}, url: {}",
            self.admin_key.as_ref().unwrap(),
            self.url.as_ref().unwrap()
        )
    }
}

#[derive(Clone)]
pub struct LnbitsLightning {
    pub client: LNBitsClient,
}

impl LnbitsLightning {
    pub fn new(admin_key: String, url: String) -> Self {
        Self {
            client: LNBitsClient::new(&admin_key, &url, None)
                .expect("Can not create Lnbits client"),
        }
    }
}

#[async_trait]
impl Lightning for LnbitsLightning {
    async fn is_invoice_paid(&self, invoice: String) -> Result<bool, MokshaMintError> {
        let decoded_invoice = self.decode_invoice(invoice).await?;
        Ok(self
            .client
            .is_invoice_paid(&decoded_invoice.payment_hash().to_string())
            .await?)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        Ok(self
            .client
            .create_invoice(&CreateInvoiceParams {
                amount,
                unit: "sat".to_string(),
                memo: None,
                expiry: Some(10000),
                webhook: None,
                internal: None,
            })
            .await?)
    }

    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError> {
        self.client
            .pay_invoice(&payment_request)
            .await
            .map_err(|err| MokshaMintError::PayInvoice(payment_request, err))
    }
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
    ) -> Result<Self, LightningError> {
        let lnbits_url = Url::parse(lnbits_url)?;

        let reqwest_client = {
            if let Some(tor_socket) = tor_socket {
                let proxy = reqwest::Proxy::all(tor_socket).expect("tor proxy should be there");
                reqwest::Client::builder().proxy(proxy).build()?
            } else {
                reqwest::Client::builder().build()?
            }
        };

        Ok(Self {
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
        let payment_hash = serde_json::from_str::<serde_json::Value>(&body)?["payment_hash"]
            .as_str()
            .expect("payment_hash is empty")
            .to_owned();
        Ok(PayInvoiceResult {
            payment_hash,
            total_fees: 0,
        })
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

#[cfg(test)]
mod tests {
    use crate::lightning::lnbits::LnbitsLightning;
    use crate::lightning::Lightning;

    #[tokio::test]
    async fn test_decode_invoice() -> anyhow::Result<()> {
        let invoice = "lnbcrt55550n1pjga687pp5ac8ja6n5hn90huztxxp746w48vtj8ys5uvze6749dvcsd5j5sdvsdqqcqzzsxqyz5vqsp5kzzq0ycxspxjygsxkfkexkkejjr5ggeyl56mwa7s0ygk2q8z92ns9qyyssqt7myq7sryffasx8v47al053ut4vqts32e9hvedvs7eml5h9vdrtj3k5m72yex5jv355jpuzk2xjjn5468cz87nhp50jyr2al2a5zjvgq2xs5uq".to_string();

        let lightning =
            LnbitsLightning::new("admin_key".to_string(), "http://localhost:5000".to_string());

        let decoded_invoice = lightning.decode_invoice(invoice).await?;
        assert_eq!(
            decoded_invoice
                .amount_milli_satoshis()
                .expect("invalid amount"),
            5_555 * 1_000
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_decode_invoice_invalid() -> anyhow::Result<()> {
        let invoice = "lnbcrt55550n1pjga689pp5ac8ja6n5hn90huztyxp746w48vtj8ys5uvze6749dvcsd5j5sdvsdqqcqzzsxqyz5vqsp5kzzq0ycxspxjygsxkfkexkkejjr5ggeyl56mwa7s0ygk2q8z92ns9qyyssqt7myq7sryffasx8v47al053ut4vqts32e9hvedvs7eml5h9vdrtj3k5m72yex5jv355jpuzk2xjjn5468cz87nhp50jyr2al2a5zjvgq2xs5uw".to_string();

        let lightning =
            LnbitsLightning::new("admin_key".to_string(), "http://localhost:5000".to_string());

        let decoded_invoice = lightning.decode_invoice(invoice).await;
        assert!(decoded_invoice.is_err());
        Ok(())
    }
}
