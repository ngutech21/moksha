// This is the entry point of your Rust library.
// When adding new code to your project, note that only items used
// here will be transformed to their Dart equivalents.

use std::io::Error;

use cashurs_wallet::client::Client;
use cashurs_wallet::localstore::LocalStore;
use cashurs_wallet::{client::HttpClient, localstore::SqliteLocalStore, wallet};
use tokio::runtime::Runtime;

pub fn say_hello() -> String {
    "Hello from Rust!".to_string()
}

pub fn generate_qrcode(amount: u8) -> anyhow::Result<String> {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let result = internal_generate_qrcode(amount).await;
        result.map_err(anyhow::Error::from)
    })
}

async fn internal_generate_qrcode(amount: u8) -> Result<String, Error> {
    Ok(format!("qr code for value {amount}"))
}

pub fn get_balance() -> anyhow::Result<u64> {
    let mint_url = "http://127.0.0.1:3338".to_string();
    let client = HttpClient::new(mint_url.clone());
    let rt = Runtime::new().expect("Could not create runtime");

    rt.block_on(async move {
        let keys = client.get_mint_keys().await.map_err(anyhow::Error::from)?;

        let keysets = client
            .get_mint_keysets()
            .await
            .map_err(anyhow::Error::from)?;
        println!("get_balance() localstore");
        let localstore = Box::new(
            SqliteLocalStore::with_path("../data/wallet/cashurs_wallet.db".to_string())
                .await
                .map_err(anyhow::Error::from)?,
        );
        localstore.migrate().await;

        let wallet = wallet::Wallet::new(Box::new(client), keys, keysets, localstore, mint_url);
        let balance = wallet.get_balance().await?;
        Ok(balance)
    })
}
