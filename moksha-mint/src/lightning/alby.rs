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
pub struct AlbyLightningSettings {
    #[clap(long, env = "MINT_ALBY_API_KEY")]
    pub api_key: Option<String>,
}

impl fmt::Display for AlbyLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "api_key: {}", self.api_key.as_ref().unwrap(),)
    }
}

impl AlbyLightningSettings {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: Some(api_key.to_owned()),
        }
    }
}

#[derive(Clone)]
pub struct AlbyLightning {
    pub client: AlbyClient,
}

impl AlbyLightning {
    pub fn new(api_key: String) -> Self {
        Self {
            client: AlbyClient::new(&api_key).expect("Can not create Alby client"),
        }
    }
}
#[async_trait]
impl Lightning for AlbyLightning {
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
pub struct AlbyClient {
    api_key: String,
    alby_url: Url,
    reqwest_client: reqwest::Client,
}

impl AlbyClient {
    pub fn new(api_key: &str) -> Result<Self, LightningError> {
        let alby_url = Url::parse("https://api.getalby.com")?;

        let reqwest_client = reqwest::Client::builder().build()?;

        Ok(Self {
            api_key: api_key.to_owned(),
            alby_url,
            reqwest_client,
        })
    }
}

impl AlbyClient {
    pub async fn make_get(&self, endpoint: &str) -> Result<String, LightningError> {
        let url = self.alby_url.join(endpoint)?;
        let response = self
            .reqwest_client
            .get(url)
            .bearer_auth(self.api_key.clone())
            .send()
            .await?;

        // Alby API returns a 404 for invoices that aren't settled yet
        // if response.status() == reqwest::StatusCode::NOT_FOUND {
        //     return Err(LightningError::NotFound);
        // }

        Ok(response.text().await?)
    }

    pub async fn make_post(&self, endpoint: &str, body: &str) -> Result<String, LightningError> {
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
            return Err(LightningError::NotFound);
        }

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LightningError::Unauthorized);
        }

        Ok(response.text().await?)
    }
}

impl AlbyClient {
    pub async fn create_invoice(
        &self,
        params: &CreateInvoiceParams,
    ) -> Result<CreateInvoiceResult, LightningError> {
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

    pub async fn pay_invoice(&self, bolt11: &str) -> Result<PayInvoiceResult, LightningError> {
        let body = self
            .make_post(
                "payments/bolt11",
                &serde_json::to_string(&serde_json::json!({ "invoice": bolt11 }))?,
            )
            .await?;

        let response: serde_json::Value = serde_json::from_str(&body)?;

        Ok(PayInvoiceResult {
            payment_hash: response["payment_hash"]
                .as_str()
                .expect("payment_hash is empty")
                .to_owned(),
            total_fees: 0, // FIXME alby does not return fees at the moment
        })
    }

    pub async fn is_invoice_paid(&self, payment_hash: &str) -> Result<bool, LightningError> {
        let body = self.make_get(&format!("invoices/{payment_hash}")).await?;
        Ok(serde_json::from_str::<serde_json::Value>(&body)?["settled"]
            .as_bool()
            .unwrap_or(false))
    }
}
