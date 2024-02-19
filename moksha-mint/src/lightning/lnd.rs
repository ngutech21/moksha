use std::{
    fmt::{self, Formatter},
    path::PathBuf,
    sync::Arc,
};

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceResult, PayInvoiceResult},
    url_serialize::{deserialize_url, serialize_url},
};
use async_trait::async_trait;
use clap::Parser;
use fedimint_tonic_lnd::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use tracing::info;
use url::Url;

use super::Lightning;

#[derive(Deserialize, Serialize, Debug, Clone, Default, Parser)]
pub struct LndLightningSettings {
    #[clap(long, env = "MINT_LND_GRPC_HOST")]
    #[serde(serialize_with = "serialize_url", deserialize_with = "deserialize_url")]
    pub grpc_host: Option<Url>,

    #[clap(long, env = "MINT_LND_TLS_CERT_PATH")]
    pub tls_cert_path: Option<PathBuf>,

    #[clap(long, env = "MINT_LND_MACAROON_PATH")]
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
                .unwrap() // FIXME unwrap
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
        let client =
            fedimint_tonic_lnd::connect(address.to_string(), cert_file, &macaroon_file).await;

        Ok(Self(Arc::new(Mutex::new(
            client.map_err(MokshaMintError::ConnectError)?,
        ))))
    }

    pub async fn client_lock(
        &self,
    ) -> Result<MappedMutexGuard<'_, fedimint_tonic_lnd::LightningClient>, MokshaMintError> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.lightning()))
    }
}

#[allow(implied_bounds_entailment)]
#[async_trait]
impl Lightning for LndLightning {
    async fn is_invoice_paid(&self, payment_request: String) -> Result<bool, MokshaMintError> {
        let invoice = self.decode_invoice(payment_request).await?;
        let payment_hash = invoice.payment_hash();
        let invoice_request = fedimint_tonic_lnd::lnrpc::PaymentHash {
            r_hash: payment_hash.to_vec(),
            ..Default::default()
        };

        let invoice = self
            .client_lock()
            .await?
            .lookup_invoice(fedimint_tonic_lnd::tonic::Request::new(invoice_request))
            .await?
            .into_inner();

        Ok(invoice.state == fedimint_tonic_lnd::lnrpc::invoice::InvoiceState::Settled as i32)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        let invoice_request = fedimint_tonic_lnd::lnrpc::Invoice {
            value: amount as i64,
            ..Default::default()
        };

        let invoice = self
            .client_lock()
            .await?
            .add_invoice(fedimint_tonic_lnd::tonic::Request::new(invoice_request))
            .await?
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
        let pay_req = fedimint_tonic_lnd::lnrpc::SendRequest {
            payment_request,
            ..Default::default()
        };
        let payment_response = self
            .client_lock()
            .await?
            .send_payment_sync(fedimint_tonic_lnd::tonic::Request::new(pay_req))
            .await?
            .into_inner();

        let total_fees = payment_response
            .payment_route
            .map_or(0, |route| route.total_fees_msat / 1_000) as u64;

        info!("lnd total_fees: {}", total_fees);

        Ok(PayInvoiceResult {
            payment_hash: hex::encode(payment_response.payment_hash),
            total_fees,
        })
    }
}
