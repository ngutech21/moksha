use fedimint_tonic_lnd::{
    lnrpc::{ConnectPeerRequest, GetInfoRequest, ListChannelsRequest, ListPeersRequest},
    walletrpc::ListUnspentRequest,
    Client,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use url::Url;

use fedimint_tonic_lnd::lnrpc::{AddressType, NewAddressRequest};

pub struct LndClient(Arc<Mutex<Client>>);

pub const LND_MINT_ADDRESS: &str = "https://localhost:11001";
pub const LND_WALLET_ADDRESS: &str = "https://localhost:12001";

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

    pub async fn new_mint_lnd() -> anyhow::Result<Self> {
        let url = Url::parse(LND_MINT_ADDRESS)?;
        let project_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let cert_file: PathBuf = (project_dir.clone() + "/../data/lnd-mint/tls.cert").into();
        let macaroon_file: PathBuf =
            (project_dir + "/../data/lnd-mint/data/chain/bitcoin/regtest/admin.macaroon").into();
        Self::new(url, &cert_file, &macaroon_file).await
    }

    pub async fn new_wallet_lnd() -> anyhow::Result<Self> {
        let url = Url::parse(LND_WALLET_ADDRESS)?;
        let project_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let cert_file: PathBuf = (project_dir.clone() + "/../data/lnd-wallet/tls.cert").into();
        let macaroon_file: PathBuf =
            (project_dir + "/../data/lnd-wallet/data/chain/bitcoin/regtest/admin.macaroon").into();
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

    async fn is_node_synced(&self) -> anyhow::Result<bool> {
        let mut client = self.client_lock().await?;
        let info = client
            .get_info(GetInfoRequest::default())
            .await?
            .into_inner();
        Ok(info.synced_to_chain)
    }

    pub async fn wait_for_node_sync(&self) -> anyhow::Result<()> {
        loop {
            if self.is_node_synced().await? {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
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

    pub async fn connect_to_peer(&self, peer_pubkey: &str, host_port: &str) -> anyhow::Result<()> {
        self.wait_for_node_sync().await?;
        let mut client = self.client_lock().await?;

        let peers = client.list_peers(ListPeersRequest::default()).await?;
        let peer_already_exists = peers
            .into_inner()
            .peers
            .iter()
            .any(|peer| peer.pub_key == peer_pubkey);
        if peer_already_exists {
            return Ok(());
        }

        let request = ConnectPeerRequest {
            addr: Some(fedimint_tonic_lnd::lnrpc::LightningAddress {
                pubkey: peer_pubkey.to_string(),
                host: host_port.to_string(),
            }),
            ..Default::default()
        };
        client.connect_peer(request).await?;
        Ok(())
    }

    pub async fn open_channel(&self, peer_pubkey: &str, amount: u64) -> anyhow::Result<bool> {
        self.wait_for_node_sync().await?;
        let mut client = self.client_lock().await?;

        let open_channels_with_peer = client
            .list_channels(ListChannelsRequest {
                peer: hex::decode(peer_pubkey)?,
                ..Default::default()
            })
            .await?
            .into_inner()
            .channels
            .into_iter()
            .collect::<Vec<_>>();
        if !open_channels_with_peer.is_empty() {
            return Ok(false);
        }

        let request = fedimint_tonic_lnd::lnrpc::OpenChannelRequest {
            node_pubkey: hex::decode(peer_pubkey)?,
            local_funding_amount: amount as i64,
            zero_conf: false,
            min_confs: 0,
            sat_per_vbyte: 100,
            push_sat: amount as i64 / 2,
            ..Default::default()
        };
        client.open_channel(request).await?;
        Ok(true)
    }

    pub async fn get_pubkey(&self) -> anyhow::Result<String> {
        let mut client = self.client_lock().await?;
        let request = GetInfoRequest::default();

        let response = client.get_info(request).await?.into_inner();
        Ok(response.identity_pubkey)
    }

    pub async fn create_invoice(&self, amount: u64) -> anyhow::Result<String> {
        let mut client = self.client_lock().await?;
        let request = fedimint_tonic_lnd::lnrpc::Invoice {
            value: amount as i64,
            ..Default::default()
        };

        let response = client.add_invoice(request).await?.into_inner();
        Ok(response.payment_request)
    }

    pub async fn pay_invoice(&self, payment_request: &str) -> anyhow::Result<()> {
        self.client_lock()
            .await?
            .send_payment_sync(fedimint_tonic_lnd::tonic::Request::new(
                fedimint_tonic_lnd::lnrpc::SendRequest {
                    payment_request: payment_request.to_string(),
                    ..Default::default()
                },
            ))
            .await?
            .into_inner();

        Ok(())
    }
}
