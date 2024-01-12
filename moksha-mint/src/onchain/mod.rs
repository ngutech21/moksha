use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use fedimint_tonic_lnd::{
    lnrpc::{AddressType, EstimateFeeRequest, NewAddressRequest, SendCoinsRequest},
    Client,
};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use url::Url;

use crate::error::MokshaMintError;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Onchain: Send + Sync {
    async fn new_address(&self) -> Result<String, MokshaMintError>;
    async fn send_coins(
        &self,
        address: &str,
        amount: u64,
        sat_per_vbyte: u32,
    ) -> Result<SendCoinsResult, MokshaMintError>;

    async fn estimate_fee(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<EstimateFeeResult, MokshaMintError>;

    async fn is_paid(&self, address: &str, amount: u64) -> Result<bool, MokshaMintError>;
}

#[derive(Debug, Clone)]
pub struct EstimateFeeResult {
    pub sat_per_vbyte: u64,
}

#[derive(Debug, Clone)]
pub struct SendCoinsResult {
    pub txid: String,
}

pub struct LndOnchain(Arc<Mutex<Client>>);

impl LndOnchain {
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
    ) -> anyhow::Result<MappedMutexGuard<'_, fedimint_tonic_lnd::LightningClient>> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.lightning()))
    }
}

#[async_trait]
impl Onchain for LndOnchain {
    async fn is_paid(&self, address: &str, amount: u64) -> Result<bool, MokshaMintError> {
        Ok(false)
    }

    async fn new_address(&self) -> Result<String, MokshaMintError> {
        let mut client = self.client_lock().await.expect("failed to lock client");
        let response = client.new_address(NewAddressRequest {
            r#type: AddressType::WitnessPubkeyHash as i32,
            ..Default::default()
        });
        Ok(response
            .await
            .expect("failed to create address")
            .into_inner()
            .address)
    }

    async fn send_coins(
        &self,
        address: &str,
        amount: u64,
        sat_per_vbyte: u32,
    ) -> Result<SendCoinsResult, MokshaMintError> {
        let mut client = self.client_lock().await.expect("failed to local client");
        let response = client
            .send_coins(SendCoinsRequest {
                addr: address.to_owned(),
                amount: amount as i64,
                sat_per_vbyte: sat_per_vbyte as u64,
                ..Default::default()
            })
            .await
            .expect("failed to send coins");

        Ok(SendCoinsResult {
            txid: response.into_inner().txid,
        })
    }

    async fn estimate_fee(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<EstimateFeeResult, MokshaMintError> {
        let mut client = self.client_lock().await.expect("failed to local client");
        let response = client
            .estimate_fee(EstimateFeeRequest {
                addr_to_amount: [(address.to_owned(), amount as i64)]
                    .iter()
                    .cloned()
                    .collect::<HashMap<_, _>>(),
                target_conf: 1,
                ..Default::default()
            })
            .await
            .expect("failed to estimate fee");

        Ok(EstimateFeeResult {
            sat_per_vbyte: response.into_inner().sat_per_vbyte,
        })
    }
}
