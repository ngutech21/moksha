use bitcoincore_rpc::{
    bitcoin::{Address, Amount},
    json::AddressType,
    Auth, Client, RpcApi,
};
use std::{str::FromStr, time::Duration};

pub struct BitcoinClient {
    pub client: Client,
}

impl BitcoinClient {
    pub async fn new_local() -> anyhow::Result<Self> {
        let client = Client::new(
            "http://localhost:18453/",
            Auth::UserPass("polaruser".to_string(), "polarpass".to_string()),
        )?;

        let wallets = client.list_wallets()?;
        if wallets.is_empty() {
            Self::create_wallet_autoload().await?;
        }
        Ok(Self { client })
    }

    // creates a wallet and loads it on startup
    pub async fn create_wallet_autoload() -> anyhow::Result<()> {
        let _ = reqwest::Client::new()
            .post("http://localhost:18453/")
            .basic_auth("polaruser", Some("polarpass"))
            .body(
                r#"{"jsonrpc": "1.0", "method": "createwallet", "params": ["testwallet", null, null, null, null, null, true]}"#
                    .to_string(),
            )
            .send()
            .await?;
        Ok(())
    }

    pub fn get_block_height(&self) -> anyhow::Result<u64> {
        Ok(self.client.get_block_count()?)
    }

    pub fn get_new_address(&self) -> anyhow::Result<String> {
        let new_address = self
            .client
            .get_new_address(None, Some(AddressType::Bech32))?;
        Ok(new_address.assume_checked().to_string())
    }

    pub async fn mine_blocks(&self, block_num: u64) -> anyhow::Result<()> {
        let new_adr = self.get_new_address()?;
        self.generate_to_address(block_num, &new_adr).await?;
        Ok(())
    }

    pub async fn generate_to_address(&self, block_num: u64, address: &str) -> anyhow::Result<()> {
        let adr = Address::from_str(address)?;
        let adr = adr.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;
        let _old_block_height = self.client.get_block_count()?;
        let _hashes = self.client.generate_to_address(block_num, &adr)?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    pub async fn send_to_address(&self, address: &str, amount: Amount) -> anyhow::Result<()> {
        let adr = Address::from_str(address)?;
        let adr = adr.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;
        self.client.send_to_address(
            &adr,
            amount,
            None,
            None,
            Some(false),
            Some(false),
            None,
            None,
        )?;

        self.mine_blocks(10).await?;
        Ok(())
    }

    pub fn get_balance(&self) -> anyhow::Result<Amount> {
        Ok(self.client.get_balance(None, None)?)
    }
}
