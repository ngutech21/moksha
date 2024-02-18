use super::{BtcOnchain, EstimateFeeResult, SendCoinsResult};
use crate::error::MokshaMintError;
use async_trait::async_trait;
use fedimint_tonic_lnd::{
    lnrpc::{AddressType, EstimateFeeRequest, NewAddressRequest, SendCoinsRequest},
    walletrpc::ListUnspentRequest,
    Client,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use url::Url;

pub struct LndBtcOnchain(Arc<Mutex<Client>>);

impl LndBtcOnchain {
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

    pub async fn wallet_lock(
        &self,
    ) -> Result<MappedMutexGuard<'_, fedimint_tonic_lnd::WalletKitClient>, MokshaMintError> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.wallet()))
    }
}

#[async_trait]
impl BtcOnchain for LndBtcOnchain {
    async fn is_transaction_paid(&self, txid: &str) -> Result<bool, MokshaMintError> {
        let request = ListUnspentRequest {
            min_confs: 0,
            max_confs: i32::MAX,
            ..Default::default()
        };

        let response = self.wallet_lock().await?.list_unspent(request).await?;

        Ok(response
            .get_ref()
            .utxos
            .iter()
            .any(|utxo| utxo.outpoint.clone().unwrap().txid_str == txid && utxo.confirmations > 0))
    }

    async fn is_paid(
        &self,
        address: &str,
        amount: u64,
        min_confirmations: u8,
    ) -> Result<bool, MokshaMintError> {
        let request = ListUnspentRequest {
            min_confs: 0,
            max_confs: i32::MAX,
            ..Default::default()
        };

        let response = self.wallet_lock().await?.list_unspent(request).await?;

        Ok(response.get_ref().utxos.iter().any(|utxo| {
            utxo.address == address
                && utxo.amount_sat >= amount as i64
                && utxo.confirmations >= min_confirmations as i64
        }))
    }

    async fn new_address(&self) -> Result<String, MokshaMintError> {
        let mut client = self.client_lock().await?;
        let response = client
            .new_address(NewAddressRequest {
                r#type: AddressType::WitnessPubkeyHash as i32,
                ..Default::default()
            })
            .await?
            .into_inner();

        Ok(response.address)
    }

    async fn send_coins(
        &self,
        address: &str,
        amount: u64,
        sat_per_vbyte: u32,
    ) -> Result<SendCoinsResult, MokshaMintError> {
        let response = self
            .client_lock()
            .await?
            .send_coins(SendCoinsRequest {
                addr: address.to_owned(),
                amount: amount as i64,
                sat_per_vbyte: sat_per_vbyte as u64,
                ..Default::default()
            })
            .await?
            .into_inner();

        Ok(SendCoinsResult {
            txid: response.txid,
        })
    }

    async fn estimate_fee(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<EstimateFeeResult, MokshaMintError> {
        let response = self
            .client_lock()
            .await?
            .estimate_fee(EstimateFeeRequest {
                addr_to_amount: std::iter::once(&(address.to_owned(), amount as i64))
                    .cloned()
                    .collect::<HashMap<_, _>>(),
                target_conf: 1,
                ..Default::default()
            })
            .await?
            .into_inner();

        Ok(EstimateFeeResult {
            fee_in_sat: response.fee_sat as u64,
            sat_per_vbyte: response.sat_per_vbyte as u32,
        })
    }
}
