use async_trait::async_trait;
use fedimint_tonic_lnd::Client;
use std::fmt::{self, Formatter};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

use url::Url;

use serde::{Deserialize, Serialize};

use crate::url_serialize::{deserialize_url, serialize_url};

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceParams, CreateInvoiceResult, PayInvoiceResult},
};

use lightning_invoice::{Bolt11Invoice as LNInvoice, SignedRawBolt11Invoice};

mod alby;
pub mod cln;
pub mod error;
mod lnbits;
pub mod lnd;
mod strike;

#[cfg(test)]
use mockall::automock;
use std::{path::PathBuf, str::FromStr, sync::Arc};

use self::cln::ClnLightningSettings;
use self::lnd::LndLightningSettings;
use self::{alby::AlbyClient, error::LightningError, lnbits::LNBitsClient, strike::StrikeClient};

#[derive(Debug, Clone)]
pub enum LightningType {
    Lnbits(LnbitsLightningSettings),
    Alby(AlbyLightningSettings),
    Strike(StrikeLightningSettings),
    Lnd(LndLightningSettings),
    Cln(ClnLightningSettings),
}

impl fmt::Display for LightningType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lnbits(settings) => write!(f, "Lnbits: {}", settings),
            Self::Alby(settings) => write!(f, "Alby: {}", settings),
            Self::Strike(settings) => write!(f, "Strike: {}", settings),
            Self::Lnd(settings) => write!(f, "Lnd: {}", settings),
            Self::Cln(settings) => write!(f, "Cln: {}", settings),
        }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Lightning: Send + Sync {
    async fn is_invoice_paid(&self, invoice: String) -> Result<bool, MokshaMintError>;
    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError>;
    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError>;

    async fn decode_invoice(&self, payment_request: String) -> Result<LNInvoice, MokshaMintError> {
        LNInvoice::from_str(&payment_request)
            .map_err(|err| MokshaMintError::DecodeInvoice(payment_request, err))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LnbitsLightningSettings {
    pub admin_key: Option<String>,
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AlbyLightningSettings {
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct StrikeLightningSettings {
    pub api_key: Option<String>,
}

impl fmt::Display for StrikeLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "api_key: {}", self.api_key.as_ref().unwrap(),)
    }
}

impl StrikeLightningSettings {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: Some(api_key.to_owned()),
        }
    }
}

#[derive(Clone)]
pub struct StrikeLightning {
    pub client: StrikeClient,
}

impl StrikeLightning {
    pub fn new(api_key: String) -> Self {
        Self {
            client: StrikeClient::new(&api_key).expect("Can not create Strike client"),
        }
    }
}

#[async_trait]
impl Lightning for StrikeLightning {
    async fn is_invoice_paid(&self, invoice: String) -> Result<bool, MokshaMintError> {
        let decoded_invoice = self.decode_invoice(invoice).await?;
        let description_hash = decoded_invoice
            .into_signed_raw()
            .description_hash()
            .unwrap()
            .0;

        // invoiceId is the last 16 bytes of the description hash
        let invoice_id = format_as_uuid_string(&description_hash[16..]);

        Ok(self.client.is_invoice_paid(&invoice_id).await?)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        let strike_invoice_id = self
            .client
            .create_strike_invoice(&CreateInvoiceParams {
                amount,
                unit: "sat".to_string(),
                memo: None,
                expiry: Some(10000),
                webhook: None,
                internal: None,
            })
            .await?;

        let payment_request = self.client.create_strike_quote(&strike_invoice_id).await?;
        // strike doesn't return the payment_hash so we have to read the invoice into a Bolt11 and extract it
        let invoice =
            LNInvoice::from_signed(payment_request.parse::<SignedRawBolt11Invoice>().unwrap())
                .unwrap();
        let payment_hash = invoice.payment_hash().to_vec();

        Ok(CreateInvoiceResult {
            payment_hash,
            payment_request,
        })
    }

    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError> {
        // strike doesn't return the payment_hash so we have to read the invoice into a Bolt11 and extract it
        let invoice = self.decode_invoice(payment_request.clone()).await?;
        let payment_hash = invoice.payment_hash().to_vec();

        let payment_quote_id = self
            .client
            .create_ln_payment_quote(&invoice.into_signed_raw().to_string())
            .await?;

        let payment_result = self
            .client
            .execute_ln_payment_quote(&payment_quote_id)
            .await?;

        if !payment_result {
            return Err(MokshaMintError::PayInvoice(
                payment_request,
                LightningError::PaymentFailed,
            ));
        }

        Ok(PayInvoiceResult {
            payment_hash: hex::encode(payment_hash),
            total_fees: 0, // FIXME return fees for strike
        })
    }
}

fn format_as_uuid_string(bytes: &[u8]) -> String {
    let byte_str = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &byte_str[..8],
        &byte_str[8..12],
        &byte_str[12..16],
        &byte_str[16..20],
        &byte_str[20..]
    )
}

#[cfg(test)]
mod tests {
    use crate::lightning::Lightning;
    use crate::lightning::LnbitsLightning;

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
