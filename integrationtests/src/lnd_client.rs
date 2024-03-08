use fedimint_tonic_lnd::{walletrpc::ListUnspentRequest, Client};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use url::Url;

use fedimint_tonic_lnd::lnrpc::{AddressType, NewAddressRequest};

pub struct LndClient(Arc<Mutex<Client>>);

impl LndClient {
    pub async fn new(
        address: Url,
        cert_file: &PathBuf,
        macaroon_file: &PathBuf,
    ) -> anyhow::Result<Self> {
        let client =
            fedimint_tonic_lnd::connect(address.to_string(), cert_file, &macaroon_file).await;

        Ok(Self(Arc::new(Mutex::new(client?))))
    }

    pub async fn new_local() -> anyhow::Result<Self> {
        let url = Url::parse("https://localhost:10001").unwrap();
        let cert_file = PathBuf::from("./data/lnd1/tls.cert");
        let macaroon_file = PathBuf::from("./data/lnd1/data/chain/bitcoin/regtest/admin.macaroon");
        Self::new(url, &cert_file, &macaroon_file).await
    }

    pub async fn client_lock(
        &self,
    ) -> anyhow::Result<MappedMutexGuard<'_, fedimint_tonic_lnd::LightningClient>> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.lightning()))
    }

    pub async fn wallet_lock(
        &self,
    ) -> anyhow::Result<MappedMutexGuard<'_, fedimint_tonic_lnd::WalletKitClient>> {
        let guard = self.0.lock().await;
        Ok(MutexGuard::map(guard, |client| client.wallet()))
    }

    pub async fn new_address(&self) -> anyhow::Result<String> {
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

    pub async fn get_balance(&self, address: &str, min_confirmations: u32) -> anyhow::Result<u64> {
        let request = ListUnspentRequest {
            min_confs: 0,
            max_confs: i32::MAX,
            ..Default::default()
        };

        let response = self.wallet_lock().await?.list_unspent(request).await?;

        let amount_in_sat = response
            .get_ref()
            .utxos
            .iter()
            .filter(|utxo| {
                utxo.address == address && utxo.confirmations >= min_confirmations as i64
            })
            .map(|utxo| utxo.amount_sat)
            .sum::<i64>();

        Ok(amount_in_sat as u64)
    }
}
