use itests::setup::{read_fixture, start_mint};
use moksha_core::primitives::{CurrencyUnit, PaymentMethod};

use moksha_wallet::client::CashuClient;
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::WalletBuilder;

use mokshamint::lightning::{lnbits::LnbitsLightningSettings, LightningType};
use reqwest::Url;
use std::time::Duration;
use testcontainers::{clients, RunnableImage};
use testcontainers_modules::postgres::Postgres;
use tokio::time::{sleep_until, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
pub async fn test_bolt11_lnbitsmock() -> anyhow::Result<()> {
    // create postgres container that will be destroyed after the test is done
    let docker = clients::Cli::default();
    let node = Postgres::default().with_host_auth();
    let img = RunnableImage::from(node).with_tag("16.2-alpine");
    let node = docker.run(img);
    let host_port = node.get_host_port_ipv4(5432);

    // start lnbits
    let _lnbits_thread = tokio::spawn(async {
        let _ = itests::lnbitsmock::run_server(6100).await;
    });

    let _server_thread = tokio::spawn(async move {
        let ln = LightningType::Lnbits(LnbitsLightningSettings::new(
            "my_admin_key",
            "http://127.0.0.1:6100",
        ));

        start_mint(host_port, ln, None)
            .await
            .expect("Could not start mint server");
    });

    // Wait for the server to start
    tokio::time::sleep(Duration::from_millis(800)).await;

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

    // mint some tokens
    let mint_amount = 6_000;
    let mint_quote = wallet.create_quote_bolt11(mint_amount).await?;
    let hash = mint_quote.clone().quote;

    sleep_until(Instant::now() + Duration::from_millis(1_000)).await;
    let mint_result = wallet
        .mint_tokens(&PaymentMethod::Bolt11, mint_amount.into(), hash.clone())
        .await?;
    assert_eq!(6_000, mint_result.total_amount());

    let balance = wallet.get_balance().await?;
    assert_eq!(6_000, balance);

    // pay ln-invoice
    let invoice_1000 = read_fixture("invoice_1000.txt")?;
    let quote = wallet
        .get_melt_quote_bolt11(invoice_1000.clone(), CurrencyUnit::Sat)
        .await?;
    let result_pay_invoice = wallet.pay_invoice(&quote, invoice_1000).await;
    if result_pay_invoice.is_err() {
        println!("error in pay_invoice{:?}", result_pay_invoice);
    }
    assert!(result_pay_invoice.is_ok());
    let balance = wallet.get_balance().await?;
    assert_eq!(5_000, balance);

    // receive 10 sats
    let token_10: moksha_core::token::TokenV3 = read_fixture("token_10.cashu")?.try_into()?;
    let result_receive = wallet.receive_tokens(&token_10).await;
    assert!(result_receive.is_ok());
    let balance = wallet.get_balance().await?;
    assert_eq!(5_010, balance);

    // send 10 tokens
    let result_send = wallet.send_tokens(10).await;
    assert!(result_send.is_ok());
    assert_eq!(10, result_send.unwrap().total_amount());
    let balance = wallet.get_balance().await?;
    assert_eq!(5_000, balance);

    // get info
    let info = wallet.get_mint_info().await?;
    assert!(!info.nuts.nut4.disabled);
    Ok(())
}
