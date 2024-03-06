use fedimint_tonic_lnd::Client;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use url::Url;

use fedimint_tonic_lnd::{
    lnrpc::{AddressType, EstimateFeeRequest, NewAddressRequest, SendCoinsRequest},
    walletrpc::ListUnspentRequest,
};

pub struct LndBtcOnchain(Arc<Mutex<Client>>);

impl LndBtcOnchain {
    pub async fn new(
        address: Url,
        cert_file: &PathBuf,
        macaroon_file: &PathBuf,
    ) -> anyhow::Result<Self> {
        let client =
            fedimint_tonic_lnd::connect(address.to_string(), cert_file, &macaroon_file).await;

        Ok(Self(Arc::new(Mutex::new(client?))))
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

    async fn new_address(&self) -> anyhow::Result<String> {
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
}

#[tokio::main]
async fn main() {
    let url = Url::parse("https://localhost:10001").unwrap();
    let cert_file = PathBuf::from("./data/lnd1/tls.cert");
    let macaroon_file = PathBuf::from("./data/lnd1/data/chain/bitcoin/regtest/admin.macaroon");
    let lnd = LndBtcOnchain::new(url, &cert_file, &macaroon_file)
        .await
        .unwrap();
    println!("{:?}", lnd.new_address().await.unwrap())
}
