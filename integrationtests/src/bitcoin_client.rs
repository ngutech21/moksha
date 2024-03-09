use bitcoincore_rpc::{
    bitcoin::{Address, Amount},
    json::AddressType,
    Auth, Client, RpcApi,
};
use std::str::FromStr;

pub struct BitcoinClient {
    pub client: Client,
}

impl BitcoinClient {
    pub fn new_local() -> anyhow::Result<Self> {
        let wallet_name = "testwallet";
        let client = Client::new(
            &format!("http://localhost:18443/wallet/{}", wallet_name),
            Auth::UserPass("polaruser".to_string(), "polarpass".to_string()),
        )?;
        let wallet = client.list_wallets()?;
        if !wallet.contains(&wallet_name.to_owned()) {
            println!("create wallet");
            client.create_wallet(wallet_name, None, None, None, None)?;
        }
        Ok(Self { client })
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

    pub fn mine_blocks(&self, block_num: u64) -> anyhow::Result<()> {
        let new_adr = self.get_new_address()?;
        self.generate_to_address(block_num, &new_adr)?;
        Ok(())
    }

    pub fn generate_to_address(&self, block_num: u64, address: &str) -> anyhow::Result<()> {
        let adr = Address::from_str(address)?;
        let adr = adr.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;
        let _old_block_height = self.client.get_block_count()?;
        let _hashes = self.client.generate_to_address(block_num, &adr)?;
        // println!("old block height: {:?}", old_block_height);

        // let last_block = self
        //     .client
        //     .get_block_header_info(hashes.first().expect("no block mined"))?;

        // println!("last block: {:?}", last_block.height);

        std::thread::sleep(std::time::Duration::from_secs(1));
        // let new_block_height = self.client.get_block_count()?;
        // println!("new block height: {:?}", new_block_height);
        Ok(())
    }

    pub fn send_to_address(&self, address: &str, amount: Amount) -> anyhow::Result<()> {
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

        self.mine_blocks(1)?;
        Ok(())
    }

    pub fn get_balance(&self) -> anyhow::Result<Amount> {
        Ok(self.client.get_balance(None, None)?)
    }
}
