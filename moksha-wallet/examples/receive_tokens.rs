use std::env::temp_dir;

use moksha_core::token::TokenV3;
use moksha_wallet::{
    client::reqwest::HttpClient, localstore::sqlite::SqliteLocalStore, wallet::WalletBuilder,
};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = temp_dir().join("wallet.db").to_str().unwrap().to_string();
    let localstore = SqliteLocalStore::with_path(db_path).await?;
    let client = HttpClient::new();
    let wallet = WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .with_mint_url(Url::parse("https://mutinynet.moksha.cash:3338")?)
        .build()
        .await?;
    let tokens = TokenV3::deserialize("cashuAeyJ0b2tlbiI6IFt7InByb29mcyI6IFt7ImlkIjogIjAwOTkxZjRmMjc3MzMzOGMiLCAiYW1vdW50IjogMiwgInNlY3JldCI6ICI5ZmFjZWE0Y2QzN2I3ZWRlOGE4NmQzYWY1ZWIxZTczNzIxMDNmZDE2YTQ1M2E5NDQ5YjE0MDFkZDhhMzAzMWJiIiwgIkMiOiAiMDM2ZTVhOWJhOWE1ZjYxZmQ5MTk3YzM2OTgzZjc1YzAzYTUyYzc0YTJmZmM2NTBmNzg5MjJlMDcyZWY1MTI0YjZlIn1dLCAibWludCI6ICJodHRwczovL21pbnQubXV0aW55bmV0Lm1va3NoYS5jYXNoOjMzMzgifV19")?;
    wallet.receive_tokens(&tokens).await?;
    let balance = wallet.get_balance().await?;
    println!("New balance: {} sats", balance);
    Ok(())
}
