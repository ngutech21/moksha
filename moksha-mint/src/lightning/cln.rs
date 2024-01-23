use std::{fmt::Formatter, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use bitcoin_hashes::Hash;
use cln_rpc::{
    primitives::{Amount, AmountOrAny},
    ClnRpc,
};
use serde_derive::{Deserialize, Serialize};
use std::fmt::{self};

use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceResult, PayInvoiceResult},
};

use super::Lightning;
use secp256k1::rand;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ClnLightningSettings {
    pub rpc_path: Option<PathBuf>,
}
impl fmt::Display for ClnLightningSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rpc_path: {}",
            self.rpc_path.as_ref().unwrap().to_str().unwrap_or_default()
        )
    }
}

pub struct ClnLightning(Arc<Mutex<ClnRpc>>);

impl ClnLightning {
    pub async fn new(path: &PathBuf) -> Result<Self, MokshaMintError> {
        let client = ClnRpc::new(path).await;

        Ok(Self(Arc::new(Mutex::new(
            client.map_err(MokshaMintError::ClnConnectError)?,
        ))))
    }

    pub async fn client_lock(&self) -> anyhow::Result<MappedMutexGuard<'_, ClnRpc>> {
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
            .call_typed(cln_rpc::model::requests::ListinvoicesRequest {
                invstring: Some(payment_request),
                label: None,
                payment_hash: None,
                offer_id: None,
                index: None,
                start: None,
                limit: None,
            })
            .await
            .expect("failed to lookup invoice");
        let invoice = invoices
            .invoices
            .first()
            .expect("no matching invoice found");

        Ok(invoice.status == cln_rpc::model::responses::ListinvoicesInvoicesStatus::PAID)
    }

    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError> {
        let invoice = self
            .client_lock()
            .await
            .expect("failed to lock client")
            .call_typed(cln_rpc::model::requests::InvoiceRequest {
                amount_msat: AmountOrAny::Amount(Amount::from_sat(amount)),
                description: format!("{:x}", rand::random::<u128>()),
                label: format!("{:x}", rand::random::<u128>()),
                expiry: None,
                fallbacks: None,
                preimage: None,
                cltv: None,
                deschashonly: None,
            })
            .await
            .expect("failed to create invoice");

        Ok(CreateInvoiceResult {
            payment_hash: invoice.payment_hash.to_byte_array().to_vec(),
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
            .call_typed(cln_rpc::model::requests::PayRequest {
                bolt11: payment_request,
                amount_msat: None,
                label: None,
                riskfactor: None,
                maxfeepercent: None,
                retry_for: None,
                maxdelay: None,
                exemptfee: None,
                localinvreqid: None,
                exclude: None,
                maxfee: None,
                description: None,
            })
            .await
            .expect("failed to pay invoice");

        Ok(PayInvoiceResult {
            payment_hash: hex::encode(payment.payment_hash),
            total_fees: payment.amount_sent_msat.msat() - payment.amount_msat.msat(), // FIXME check if this is correct
        })
    }
}
