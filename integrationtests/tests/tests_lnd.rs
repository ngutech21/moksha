use std::time::Duration;

use itests::{
    bitcoin_client::BitcoinClient,
    lnd_client::{self, LndClient},
    setup::{fund_mint_lnd, open_channel_with_wallet, start_mint},
};
use moksha_core::primitives::PaymentMethod;
use moksha_core::{amount::Amount, primitives::CurrencyUnit};

use moksha_wallet::client::CashuClient;
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::WalletBuilder;

use mokshamint::{
    config::{BtcOnchainConfig, BtcOnchainType},
    lightning::{lnd::LndLightningSettings, LightningType},
};
use reqwest::Url;

use testcontainers::{clients, RunnableImage};
use testcontainers_modules::postgres::Postgres;
use tokio::time::{sleep_until, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_btc_onchain_mint_melt() -> anyhow::Result<()> {
    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    fund_mint_lnd(2_000_000).await?;

    // start mint server
    tokio::spawn(async move {
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_MINT_ADDRESS.parse().expect("invalid url"),
            "../data/lnd-mint/tls.cert".into(),
            "../data/lnd-mint/data/chain/bitcoin/regtest/admin.macaroon".into(),
        );

        let ln_type = LightningType::Lnd(lnd_settings.clone());

        let onchain = Some(BtcOnchainConfig {
            onchain_type: Some(BtcOnchainType::Lnd(lnd_settings)),
            ..Default::default()
        });

        start_mint(host_port, ln_type, onchain)
            .await
            .expect("Could not start mint server");
    });

    // Wait for the server to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let keys = client.get_keys(&mint_url, "sat".to_string()).await;
    assert!(keys.is_ok());

    let keysets = client.get_keysets(&mint_url, "sat".to_string()).await;
    assert!(keysets.is_ok());

    // create wallet
    let localstore = SqliteLocalStore::with_in_memory().await?;
    let wallet = WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await?;
    let wallet_keysets = wallet.add_mint_keysets(&mint_url).await?;
    let wallet_keyset = wallet_keysets.first().expect("No keyset found");

    // get initial balance
    let balance = wallet.get_balance().await?;
    assert_eq!(0, balance, "Initial balance should be 0");

    // mint 6_000 sats bitcoin onchain
    let mint_amount = 60_000;
    let mint_quote = wallet.create_quote_onchain(&mint_url, mint_amount).await?;

    let btc_client = BitcoinClient::new_local().await?;
    btc_client
        .send_to_address(
            &mint_quote.address,
            bitcoincore_rpc::bitcoin::Amount::from_sat(mint_amount),
        )
        .await?;

    let _mint_response = wallet
        .mint_tokens(
            wallet_keyset,
            &PaymentMethod::BtcOnchain,
            Amount(mint_amount),
            mint_quote.quote,
        )
        .await?;
    let balance = wallet.get_balance().await?;
    assert_eq!(mint_amount, balance);

    let btc_address = btc_client.get_new_address()?;

    let melt_amount = 21_000;
    let melt_quotes = wallet
        .get_melt_quote_btconchain(&mint_url, btc_address.clone(), melt_amount)
        .await?;

    let first_quote = melt_quotes.first().expect("No quote returned from mint");
    let result = wallet.pay_onchain(wallet_keyset, first_quote).await?;
    assert!(!result.paid);
    btc_client.mine_blocks(1).await?;

    let is_tx_paid = wallet.is_onchain_tx_paid(&mint_url, result.txid).await?;
    assert!(is_tx_paid);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_bolt11_mint() -> anyhow::Result<()> {
    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    fund_mint_lnd(2_000_000).await?;
    open_channel_with_wallet(500_000).await?;

    // start mint server
    tokio::spawn(async move {
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_MINT_ADDRESS.parse().expect("invalid url"),
            "../data/lnd-mint/tls.cert".into(),
            "../data/lnd-mint/data/chain/bitcoin/regtest/admin.macaroon".into(),
        );

        start_mint(host_port, LightningType::Lnd(lnd_settings.clone()), None)
            .await
            .expect("Could not start mint server");
    });

    // Wait for the server to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let keys = client.get_keys(&mint_url, "sat".to_string()).await;
    assert!(keys.is_ok());

    let keysets = client.get_keysets(&mint_url, "sat".to_string()).await;
    assert!(keysets.is_ok());

    // create wallet
    let tmp_dir = tempfile::tempdir()?;
    let wallet_file = tmp_dir.path().join("wallet.db");
    let localstore = SqliteLocalStore::with_path(wallet_file.to_str().unwrap().to_owned()).await?;
    let wallet = WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await?;
    let wallet_keysets = wallet.add_mint_keysets(&mint_url).await?;
    let wallet_keyset = wallet_keysets.first().unwrap(); // FIXME

    // get initial balance
    let balance = wallet.get_balance().await?;
    assert_eq!(0, balance, "Initial balance should be 0");

    // mint some tokens
    let mint_amount = 6_000;
    let mint_quote = wallet.create_quote_bolt11(&mint_url, mint_amount).await?;
    let hash = mint_quote.clone().quote;

    sleep_until(Instant::now() + Duration::from_millis(1_000)).await;
    let mint_result = wallet
        .mint_tokens(
            wallet_keyset,
            &PaymentMethod::Bolt11,
            mint_amount.into(),
            hash.clone(),
        )
        .await?;
    assert_eq!(6_000, mint_result.total_amount());

    let balance = wallet.get_balance().await?;
    assert_eq!(6_000, balance);

    // pay ln-invoice
    let wallet_lnd = LndClient::new_wallet_lnd().await?;
    let invoice_1000 = wallet_lnd.create_invoice(1_000).await?;
    let quote = wallet
        .get_melt_quote_bolt11(&mint_url, invoice_1000.clone(), CurrencyUnit::Sat)
        .await?;
    let result_pay_invoice = wallet
        .pay_invoice(wallet_keyset, &quote, invoice_1000)
        .await;
    if result_pay_invoice.is_err() {
        println!("error in pay_invoice{:?}", result_pay_invoice);
    }
    assert!(result_pay_invoice.is_ok());
    let balance = wallet.get_balance().await?;
    assert_eq!(5_000, balance);

    // send tokens
    let exported_tokens = wallet.send_tokens(wallet_keyset, 100).await?;
    assert_eq!(100, exported_tokens.total_amount());
    let balance = wallet.get_balance().await?;
    assert_eq!(4_900, balance);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_bolt11_send() -> anyhow::Result<()> {
    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    fund_mint_lnd(2_000_000).await?;
    open_channel_with_wallet(500_000).await?;

    // start mint server
    tokio::spawn(async move {
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_MINT_ADDRESS.parse().expect("invalid url"),
            "../data/lnd-mint/tls.cert".into(),
            "../data/lnd-mint/data/chain/bitcoin/regtest/admin.macaroon".into(),
        );

        start_mint(host_port, LightningType::Lnd(lnd_settings.clone()), None)
            .await
            .expect("Could not start mint server");
    });

    // Wait for the server to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let keys = client.get_keys(&mint_url, "sat".to_string()).await;
    assert!(keys.is_ok());

    let keysets = client.get_keysets(&mint_url, "sat".to_string()).await;
    assert!(keysets.is_ok());

    // create wallet
    let tmp_dir = tempfile::tempdir()?;
    let wallet_file = tmp_dir.path().join("wallet.db");
    let localstore = SqliteLocalStore::with_path(wallet_file.to_str().unwrap().to_owned()).await?;
    let wallet = WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await?;
    let wallet_keysets = wallet.add_mint_keysets(&mint_url).await?;
    let wallet_keyset = wallet_keysets.first().unwrap(); // FIXME

    // get initial balance
    let balance = wallet.get_balance().await?;
    assert_eq!(0, balance, "Initial balance should be 0");

    // mint some tokens
    let mint_amount = 2_000;
    let mint_quote = wallet.create_quote_bolt11(&mint_url, mint_amount).await?;
    let hash = mint_quote.clone().quote;

    sleep_until(Instant::now() + Duration::from_millis(1_000)).await;
    let mint_result = wallet
        .mint_tokens(
            wallet_keyset,
            &PaymentMethod::Bolt11,
            mint_amount.into(),
            hash.clone(),
        )
        .await?;
    assert_eq!(2_000, mint_result.total_amount());

    let balance = wallet.get_balance().await?;
    assert_eq!(2_000, balance);

    // send tokens
    let exported_tokens = wallet.send_tokens(wallet_keyset, 100).await?;
    assert_eq!(100, exported_tokens.total_amount());
    let balance = wallet.get_balance().await?;
    assert_eq!(1_900, balance);
    Ok(())
}
