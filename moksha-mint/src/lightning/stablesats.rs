use async_trait::async_trait;
use axum::http::HeaderValue;

use hyper::header::CONTENT_TYPE;
use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceResult, PayInvoiceResult},
};

use super::{error::LightningError, Lightning};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct StablesatsSettings {
    pub auth_bearer: Option<String>,
    pub galoy_url: Option<String>, // FIXME use Url type instead
}

impl StablesatsSettings {
    pub fn new(auth_bearer: &str, galoy_url: &str) -> StablesatsSettings {
        StablesatsSettings {
            auth_bearer: Some(auth_bearer.to_owned()),
            galoy_url: Some(galoy_url.to_owned()),
        }
    }
}

#[derive(Clone, Debug)]
struct StablesatsLightning {
    auth_bearer: String,
    galoy_url: Url,
    usd_wallet_id: String,
    reqwest_client: reqwest::Client,
}

impl StablesatsLightning {
    pub fn new(
        auth_bearer: &str,
        galoy_url: &str,
        usd_wallet_id: &str,
    ) -> Result<StablesatsLightning, LightningError> {
        let galoy_url = Url::parse(galoy_url)?;

        let reqwest_client = reqwest::Client::builder().build()?;

        Ok(StablesatsLightning {
            auth_bearer: auth_bearer.to_owned(),
            galoy_url,
            reqwest_client,
            usd_wallet_id: usd_wallet_id.to_owned(),
        })
    }

    pub async fn make_gqlpost(&self, body: &str) -> Result<String, LightningError> {
        let response = self
            .reqwest_client
            .post(self.galoy_url.clone())
            .bearer_auth(self.auth_bearer.clone())
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

#[async_trait]
impl Lightning for StablesatsLightning {
    async fn is_invoice_paid(&self, invoice: String) -> Result<bool, MokshaMintError> {
        let input = LnInvoicePaymentStatusInput {
            payment_request: invoice,
        };
        let query = format!(
            r#"{{"query":"query LnInvoicePaymentStatus($input: LnInvoicePaymentStatusInput!) {{ lnInvoicePaymentStatus(input: $input) {{ status errors {{ message path code }} }} }}","variables":{{"input":{}}}}}"#,
            serde_json::to_string(&input).map_err(MokshaMintError::Serialization)?
        );

        let response = self
            .make_gqlpost(&query)
            .await
            .map_err(|err| MokshaMintError::PayInvoice("payment_request".to_string(), err))?;

        println!("response: {:?}", response.clone());

        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        let status = response["data"]["lnInvoicePaymentStatus"]["status"]
            .as_str()
            .unwrap()
            .to_owned();

        println!("invoice paid status: {}", status.clone());

        Ok(status == "PAID")
    }

    async fn create_invoice(
        &self,
        amount_in_usd_cent: u64,
    ) -> Result<CreateInvoiceResult, MokshaMintError> {
        let input = LnUsdInvoiceCreateInput {
            amount: amount_in_usd_cent,
            wallet_id: self.usd_wallet_id.clone(),
        };
        let query = format!(
            r#"{{"query":"mutation lnUsdInvoiceCreate($input: LnUsdInvoiceCreateInput!) {{ lnUsdInvoiceCreate(input: $input) {{ invoice {{ paymentRequest paymentHash satoshis }} }} }}","variables":{{"input":{}}}}}"#,
            serde_json::to_string(&input).map_err(MokshaMintError::Serialization)?
        );

        let response = self
            .make_gqlpost(&query)
            .await
            .map_err(|err| MokshaMintError::PayInvoice("payment_request".to_string(), err))?; // FIXME

        println!("response: {:?}", response.clone());

        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        let payment_request = response["data"]["lnUsdInvoiceCreate"]["invoice"]["paymentRequest"]
            .as_str()
            .unwrap()
            .to_owned();

        let payment_hash = response["data"]["lnUsdInvoiceCreate"]["invoice"]["paymentHash"]
            .as_str()
            .unwrap();

        let sats = response["data"]["lnUsdInvoiceCreate"]["invoice"]["satoshis"]
            .as_u64()
            .unwrap();

        println!("sats {}", sats);

        Ok(CreateInvoiceResult {
            payment_hash: payment_hash.as_bytes().to_vec(),
            payment_request,
        })
    }

    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError> {
        let invoice = self.decode_invoice(payment_request.clone()).await?;
        let payment_hash = invoice.payment_hash().to_vec();

        let input = LnInvoicePaymentSendInput {
            payment_request: payment_request.clone(),
            wallet_id: self.usd_wallet_id.clone(),
        };
        let query = format!(
            r#"{{"query":"mutation LnInvoicePaymentSend($input: LnInvoicePaymentInput!) {{ lnInvoicePaymentSend(input: $input) {{ status errors {{ message path code }} }} }}","variables":{{"input":{}}}}}"#,
            serde_json::to_string(&input).map_err(MokshaMintError::Serialization)?
        );

        println!("query: {}", query.clone());
        let response = self
            .make_gqlpost(&query)
            .await
            .map_err(|err| MokshaMintError::PayInvoice(payment_request.clone(), err))?;

        println!("response: {:?}", response.clone());

        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        let status = response["data"]["lnInvoicePaymentSend"]["status"]
            .as_str()
            .unwrap();

        if status == "SUCCESS" {
            Ok(PayInvoiceResult {
                payment_hash: hex::encode(payment_hash),
            })
        } else {
            Err(MokshaMintError::PayInvoiceStablesats(
                payment_request,
                "Error paying invoice".to_owned(),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LnInvoicePaymentSendInput {
    payment_request: String,
    wallet_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LnInvoicePaymentStatusInput {
    payment_request: String,
}

// # create invoice
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LnUsdInvoiceCreateInput {
    amount: u64,
    wallet_id: String,
}

#[cfg(test)]
mod tests {

    use super::StablesatsLightning;
    use crate::lightning::Lightning;

    #[tokio::test]
    #[ignore]
    async fn test_pay_invoice() -> anyhow::Result<()> {
        let ln =
            StablesatsLightning::new("auth bearer", "https://api.blink.sv/graphql", "wallet id")?;
        let result = ln.pay_invoice("lnbc180...".to_string()).await;
        println!("{:?}", result);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_invoice() -> anyhow::Result<()> {
        let ln =
            StablesatsLightning::new("auth bearer", "https://api.blink.sv/graphql", "wallet id")?;
        let result = ln.create_invoice(50).await?;
        println!("{:?}", result);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_is_invoice_paid() -> anyhow::Result<()> {
        let ln =
            StablesatsLightning::new("auth bearer", "https://api.blink.sv/graphql", "wallet id")?;
        let result = ln.is_invoice_paid("lnbc30...".to_owned()).await?;
        println!("{:?}", result);
        Ok(())
    }
}