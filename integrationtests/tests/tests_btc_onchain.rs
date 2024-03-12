use itests::bitcoin_client::BitcoinClient;
use itests::lnd_client::{self, LndClient};
use moksha_core::amount::Amount;
use moksha_core::primitives::PaymentMethod;

use moksha_wallet::client::CashuClient;
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::WalletBuilder;

use mokshamint::config::{BtcOnchainConfig, BtcOnchainType};

use mokshamint::lightning::lnd::LndLightningSettings;
use mokshamint::lightning::LightningType;
use mokshamint::mint::MintBuilder;
use reqwest::Url;

use testcontainers::{clients, RunnableImage};
use testcontainers_modules::postgres::Postgres;

#[tokio::test(flavor = "multi_thread")]
async fn test_integration() -> anyhow::Result<()> {
    use mokshamint::config::{DatabaseConfig, ServerConfig};

    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    // mine some blocks and send 2 m sats to lnd
    let btc_client = BitcoinClient::new_local()?;
    btc_client.mine_blocks(101)?;

    tokio::spawn(async move {
        let current_dir = std::env::current_dir().expect("msg");
        println!("current_dir: {:?}", current_dir);
        let lnd_client = LndClient::new_local().await.expect("msg");
        let lnd_address = lnd_client.new_address().await.expect("msg");
        println!("lnd address: {}", lnd_address);
        btc_client
            .send_to_address(
                &lnd_address,
                bitcoincore_rpc::bitcoin::Amount::from_sat(2_000_000),
            )
            .expect("msg");
    })
    .await?;

    tokio::spawn(async move {
        let db_config = DatabaseConfig {
            db_url: format!(
                "postgres://postgres:postgres@localhost:{}/postgres",
                host_port
            ),
            ..Default::default()
        };

        // FIXME clean up
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_ADDRESS.parse().expect("invalid url"),
            "../data/lnd1/tls.cert".into(),
            "../data/lnd1/data/chain/bitcoin/regtest/admin.macaroon".into(),
        );

        let mint = MintBuilder::new()
            .with_private_key("my_private_key".to_string())
            .with_server(Some(ServerConfig {
                host_port: "127.0.0.1:8686".parse().expect("invalid address"),
                ..Default::default()
            }))
            .with_db(Some(db_config))
            .with_lightning(LightningType::Lnd(lnd_settings.clone()))
            .with_btc_onchain(Some(BtcOnchainConfig {
                onchain_type: Some(BtcOnchainType::Lnd(lnd_settings)),
                ..Default::default()
            }))
            .with_fee(Some((0.0, 0).into()))
            .build();

        let result = mokshamint::server::run_server(
            mint.await.expect("Can not connect to lightning backend"),
        )
        .await;
        assert!(result.is_ok());
    });

    // Wait for the server to start
    std::thread::sleep(std::time::Duration::from_millis(800));

    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let keys = client.get_keys(&mint_url).await;
    assert!(keys.is_ok());

    let keysets = client.get_keysets(&mint_url).await;
    assert!(keysets.is_ok());

    // create wallet
    let tmp = tempfile::tempdir()?;
    let tmp_dir = tmp
        .path()
        .to_str()
        .expect("Could not create tmp dir for wallet");

    let localstore = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db"))
        .await
        .expect("Could not create localstore");

    let wallet = WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .with_mint_url(mint_url)
        .build()
        .await?;

    // get initial balance
    let balance = wallet.get_balance().await?;
    assert_eq!(0, balance, "Initial balance should be 0");

    // mint 6_000 sats bitcoin onchain
    let mint_amount = 60_000;
    let mint_quote = wallet.create_quote_onchain(mint_amount).await?;

    let btc_client = BitcoinClient::new_local()?;
    btc_client.send_to_address(
        &mint_quote.address,
        bitcoincore_rpc::bitcoin::Amount::from_sat(mint_amount),
    )?;

    let _mint_response = wallet
        .mint_tokens(
            &PaymentMethod::BtcOnchain,
            Amount(mint_amount),
            mint_quote.quote,
        )
        .await?;
    let balance = wallet.get_balance().await?;
    assert_eq!(mint_amount, balance);

    let btc_address = btc_client.get_new_address()?;

    let melt_amount = 3_000;
    let melt_quotes = wallet
        .get_melt_quote_btconchain(btc_address.clone(), melt_amount)
        .await?;

    let first_quote = melt_quotes.first().unwrap();
    let result = wallet.pay_onchain(first_quote).await?;
    assert!(!result.paid);
    btc_client.mine_blocks(1)?;

    let is_tx_paid = wallet.is_onchain_tx_paid(result.txid).await?;
    assert!(is_tx_paid);

    Ok(())
}
