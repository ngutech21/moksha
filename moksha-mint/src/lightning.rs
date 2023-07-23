use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use tonic_lnd::Client;
use url::Url;

use crate::{
    error::MokshaMintError,
    lnbits::{CreateInvoiceParams, CreateInvoiceResult, LNBitsClient, PayInvoiceResult},
};

use lightning_invoice::Invoice as LNInvoice;

#[cfg(test)]
use mockall::automock;
use std::{path::PathBuf, str::FromStr, sync::Arc};

#[derive(Debug, Clone)]
pub enum LightningType {
    Lnbits(LnbitsLightningSettings),
    Lnd(LndLightningSettings),
}

impl fmt::Display for LightningType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LightningType::Lnbits(settings) => write!(f, "Lnbits: {}", settings),
            LightningType::Lnd(settings) => write!(f, "Lnd: {}", settings),
        }
    }
}

#[derive(Clone)]
pub struct LnbitsLightning {
    pub client: LNBitsClient,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LnbitsLightningSettings {
    pub admin_key: Option<String>,
    pub url: Option<String>, // FIXME use Url type instead
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

impl LnbitsLightningSettings {
    pub fn new(admin_key: &str, url: &str) -> Self {
        Self {
            admin_key: Some(admin_key.to_owned()),
            url: Some(url.to_owned()),
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
pub struct LndLightningSettings {
    pub grpc_host: Option<Url>,
    pub tls_cert_path: Option<PathBuf>,
    pub macaroon_path: Option<PathBuf>,
}
impl fmt::Display for LndLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "grpc_host: {}, tls_cert_path: {}, macaroon_path: {}",
            self.grpc_host.as_ref().unwrap(),
            self.tls_cert_path
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap_or_default(),
            self.macaroon_path
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap_or_default()
        )
    }
}

pub struct LndLightning(Arc<Mutex<Client>>);

impl LndLightning {
    pub async fn new(
        address: Url,
        cert_file: &PathBuf,
        macaroon_file: &PathBuf,
    ) -> Result<Self, MokshaMintError> {
        let client = tonic_lnd::connect(address.to_string(), cert_file, &macaroon_file).await;

        Ok(Self(Arc::new(Mutex::new(
            client.map_err(MokshaMintError::ConnectError)?,
        ))))
    }

    pub async fn client_lock(
        &self,
    ) -> anyhow::Result<MappedMutexGuard<'_, tonic_lnd::LightningClient>> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.lightning()))
    }
}

#[async_trait]
impl Lightning for LndLightning {
    async fn is_invoice_paid(&self, payment_request: String) -> Result<bool, MokshaMintError> {
        let invoice = self.decode_invoice(payment_request).await?;
        let payment_hash = invoice.payment_hash();
        let invoice_request = tonic_lnd::lnrpc::PaymentHash {
            r_hash: payment_hash.to_vec(),
            ..Default::default()
        };

        let invoice = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .lookup_invoice(tonic_lnd::tonic::Request::new(invoice_request))
            .await
            .expect("failed to lookup invoice")
            .into_inner();

        Ok(invoice.state == tonic_lnd::lnrpc::invoice::InvoiceState::Settled as i32)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        let invoice_request = tonic_lnd::lnrpc::Invoice {
            value: amount as i64,
            ..Default::default()
        };

        let invoice = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .add_invoice(tonic_lnd::tonic::Request::new(invoice_request))
            .await
            .expect("failed to create invoice")
            .into_inner();

        Ok(CreateInvoiceResult {
            payment_hash: invoice.r_hash,
            payment_request: invoice.payment_request,
        })
    }

    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError> {
        let pay_req = tonic_lnd::lnrpc::SendRequest {
            payment_request,
            ..Default::default()
        };
        let payment = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .send_payment_sync(tonic_lnd::tonic::Request::new(pay_req))
            .await
            .expect("failed to pay invoice")
            .into_inner();

        Ok(PayInvoiceResult {
            payment_hash: hex::encode(payment.payment_hash),
        })
    }
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
