use async_trait::async_trait;
use clap::Parser;
use cln_grpc::pb::{amount_or_any, Amount, AmountOrAny};
use cln_grpc::pb::{listinvoices_invoices::ListinvoicesInvoicesStatus, node_client::NodeClient};
use serde::{Deserialize, Serialize};
use std::fmt::{self};
use std::{fmt::Formatter, path::PathBuf, sync::Arc};

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceResult, PayInvoiceResult},
};
use tonic::transport::{Certificate, ClientTlsConfig, Identity};

use super::Lightning;

use secp256k1::rand;
use std::fs::read;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

#[derive(Deserialize, Serialize, Debug, Clone, Default, Parser)]
pub struct ClnLightningSettings {
    #[clap(long, env = "MINT_LND_GRPC_HOST")]
    pub grpc_host: Option<String>,
    #[clap(long, env = "MINT_LND_CLIENT_CERT")]
    pub client_cert: Option<PathBuf>,
    #[clap(long, env = "MINT_LND_CLIENT_CERT")]
    pub client_key: Option<PathBuf>,
    #[clap(long, env = "MINT_LND_CA_CERT")]
    pub ca_cert: Option<PathBuf>,
}

impl fmt::Display for ClnLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ClnLightningSettings")
    }
}

pub struct ClnLightning(Arc<Mutex<NodeClient<tonic::transport::Channel>>>);

impl ClnLightning {
    pub async fn new(
        grpc_host: String,
        client_cert: &PathBuf,
        client_key: &PathBuf,
        ca_cert: &PathBuf,
    ) -> Result<Self, MokshaMintError> {
        let client_cert = read(client_cert).unwrap();
        let client_key = read(client_key).unwrap();

        let identity = Identity::from_pem(client_cert, client_key);
        let ca_cert = read(ca_cert).unwrap();
        let ca_certificate = Certificate::from_pem(ca_cert);

        let tls_config = ClientTlsConfig::new()
            .domain_name("localhost")
            .identity(identity)
            .ca_certificate(ca_certificate);
        let url = grpc_host.to_owned();

        let channel = tonic::transport::Channel::from_shared(url)
            .unwrap()
            .tls_config(tls_config)
            .unwrap()
            .connect()
            .await
            .unwrap();

        let node = NodeClient::new(channel);
        Ok(Self(Arc::new(Mutex::new(node))))
    }

    pub async fn client_lock(
        &self,
    ) -> anyhow::Result<MappedMutexGuard<'_, NodeClient<tonic::transport::Channel>>> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client))
    }
}

#[async_trait]
impl Lightning for ClnLightning {
    async fn is_invoice_paid(&self, payment_request: String) -> Result<bool, MokshaMintError> {
        let invoices = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .list_invoices(cln_grpc::pb::ListinvoicesRequest {
                invstring: Some(payment_request),
                label: None,
                payment_hash: None,
                offer_id: None,
                index: None,
                start: None,
                limit: None,
            })
            .await
            .expect("failed to lookup invoice")
            .into_inner();

        let invoice = invoices
            .invoices
            .first()
            .expect("no matching invoice found");

        Ok(invoice.status() == ListinvoicesInvoicesStatus::Paid)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        let amount_msat = Some(AmountOrAny {
            value: Some(amount_or_any::Value::Amount(Amount {
                msat: amount * 1_000,
            })),
        });
        let invoice = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .invoice(cln_grpc::pb::InvoiceRequest {
                amount_msat,
                description: format!("{:x}", rand::random::<u128>()),
                label: format!("{:x}", rand::random::<u128>()),
                expiry: None,
                fallbacks: vec![],
                preimage: None,
                cltv: None,
                deschashonly: None,
            })
            .await
            .expect("failed to create invoice")
            .into_inner();

        Ok(CreateInvoiceResult {
            payment_hash: invoice.payment_hash,
            payment_request: invoice.bolt11,
        })
    }

    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError> {
        let payment = self
            .client_lock()
            .await
            .expect("failed to lock client") //FIXME map error
            .pay(cln_grpc::pb::PayRequest {
                bolt11: payment_request,
                amount_msat: None,
                label: None,
                riskfactor: None,
                maxfeepercent: None,
                retry_for: None,
                maxdelay: None,
                exemptfee: None,
                localinvreqid: None,
                exclude: vec![],
                maxfee: None,
                description: None,
            })
            .await
            .expect("failed to pay invoice")
            .into_inner();

        Ok(PayInvoiceResult {
            payment_hash: hex::encode(payment.payment_hash),
            total_fees: payment.amount_sent_msat.unwrap().msat - payment.amount_msat.unwrap().msat, // FIXME check if this is correct
        })
    }
}

// mod tests {
//     use cln_grpc::pb::GetinfoRequest;

//     #[tokio::test]
//     async fn test_connect() -> anyhow::Result<()> {
//         let path = std::path::PathBuf::from("/Users/steffen/.polar/networks/1/volumes/c-lightning/bob/lightningd/regtest/lightning-rpc");
//         let client = super::ClnLightning::new(&path).await?;
//         let info = client
//             .client_lock()
//             .await
//             .expect("failed to lock client")
//             .getinfo(GetinfoRequest {})
//             .await?;
//         println!("{:?}", info);
//         Ok(())
//     }
// }
