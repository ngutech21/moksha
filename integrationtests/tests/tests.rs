use cashurs_wallet::client::{Client, HttpClient};
use cashursmint::MintBuilder;
use reqwest::Url;
use std::thread;
use tokio::runtime::Runtime;

#[test]
pub fn test_get_mint_keys() {
    // Create a channel to signal when the server has started
    let _server_thread = thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::tempdir().expect("Could not create tmp dir");
            let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

            let mint = MintBuilder::new()
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
    let url = Url::parse("http://127.0.0.1:8686").unwrap();
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let result = client.get_mint_keys(&url).await;
        assert!(result.is_ok());
    });
}
