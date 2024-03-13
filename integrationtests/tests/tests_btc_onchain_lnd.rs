use itests::{
    bitcoin_client::BitcoinClient,
    lnd_client,
    setup::{fund_lnd, start_mint},
};
use moksha_core::amount::Amount;
use moksha_core::primitives::PaymentMethod;

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

#[tokio::test(flavor = "multi_thread")]
async fn test_integration() -> anyhow::Result<()> {
    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    fund_lnd(2_000_000).await?;

    // start mint server
    tokio::spawn(async move {
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_ADDRESS.parse().expect("invalid url"),
            "../data/lnd1/tls.cert".into(),
            "../data/lnd1/data/chain/bitcoin/regtest/admin.macaroon".into(),
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
    std::thread::sleep(std::time::Duration::from_millis(800));

    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:8686")?;
    let keys = client.get_keys(&mint_url).await;
    assert!(keys.is_ok());

    let keysets = client.get_keysets(&mint_url).await;
    assert!(keysets.is_ok());

    // create wallet
    let localstore = SqliteLocalStore::with_in_memory().await?;
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
