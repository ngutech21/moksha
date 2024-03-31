use itests::setup::read_fixture;
use moksha_core::primitives::{CurrencyUnit, PaymentMethod};
use moksha_wallet::client::CashuClient;
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::WalletBuilder;
use std::time::Duration;

use reqwest::Url;

use tokio::time::{sleep_until, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
pub async fn test_nutshell_compatibility() -> anyhow::Result<()> {
    let client = CrossPlatformHttpClient::new();
    let mint_url = Url::parse("http://127.0.0.1:2228")?;
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

    // check if mint info is correct
    let mint_info = wallet.get_mint_info().await?;
    assert_eq!(Some("nutshell".to_owned()), mint_info.name);

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

    // pay ln-invoice (10_000 invoice + 10 sats fee_reserve / 9 sats get returned)
    let invoice_1000 = read_fixture("invoice_1000.txt")?;
    let quote = wallet
        .get_melt_quote_bolt11(invoice_1000.clone(), CurrencyUnit::Sat)
        .await?;
    assert_eq!(10, quote.fee_reserve);
    let result_pay_invoice = wallet.pay_invoice(&quote, invoice_1000).await;

    if result_pay_invoice.is_err() {
        println!("error in pay_invoice{:?}", result_pay_invoice);
    }
    assert!(result_pay_invoice.is_ok());
    assert_eq!(9, result_pay_invoice?.1);
    let balance = wallet.get_balance().await?;
    assert_eq!(4_999, balance);
    Ok(())
}
