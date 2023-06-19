use cashurs_wallet::localstore::LocalStore;
use cashurs_wallet::wallet::{self};
use cashurs_wallet::{
    client::{Client, HttpClient},
    localstore::SqliteLocalStore,
};
use cashursmint::mint::Mint;
use reqwest::Url;
use std::thread;
use tokio::runtime::Runtime;

/// starts a mint and a wallet, gets the keys and checks the local balance
#[test]
pub fn test_create_wallet() -> anyhow::Result<()> {
    // Create a channel to signal when the server has started
    let _server_thread = thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            let tmp = tempfile::tempdir().expect("Could not create tmp dir");
            let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

            let mint = Mint::builder()
                .with_private_key("my_private_key".to_string())
                .with_db(tmp_dir.to_string())
                .with_lnbits(
                    "http://127.0.0.1:5000".to_string(),
                    "my_admin_key".to_string(),
                )
                .with_fee(0.0, 0)
                .build();

            let result = cashursmint::run_server(mint, 8686).await;
            drop(tmp);
            assert!(result.is_ok());
        });
        println!("server_thread finished");
    });

    // Wait for the server to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    let client = HttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let rt = Runtime::new()?;
    rt.block_on(async move {
        let keys = client.get_mint_keys(&mint_url).await;
        assert!(keys.is_ok());
        let keys = keys.unwrap();

        let keysets = client.get_mint_keysets(&mint_url).await;
        assert!(keysets.is_ok());
        let keysets = keysets.unwrap();

        // create wallet
        let tmp = tempfile::tempdir().expect("Could not create tmp dir for wallet");
        let tmp_dir = tmp
            .path()
            .to_str()
            .expect("Could not create tmp dir for wallet");

        println!(">>>> db_path: {}", tmp_dir);

        let localstore = Box::new(
            SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db"))
                .await
                .expect("Could not create localstore"),
        );
        localstore.migrate().await;

        let wallet = wallet::Wallet::new(
            Box::new(client.clone()),
            keys,
            keysets,
            localstore.clone(),
            mint_url.clone(),
        );

        let balance = wallet.get_balance().await.expect("Could not get balance");
        assert_eq!(balance, 0);
    });

    Ok(())
}
