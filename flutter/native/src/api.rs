// This is the entry point of your Rust library.
// When adding new code to your project, note that only items used
// here will be transformed to their Dart equivalents.

use std::sync::Arc;
use std::time::Duration;

use cashurs_core::model::PaymentRequest;
use cashurs_core::model::TokenV3;
use cashurs_wallet::client::Client;
use cashurs_wallet::client::HttpClient;
use cashurs_wallet::localstore::LocalStore;
use cashurs_wallet::localstore::SqliteLocalStore;
use cashurs_wallet::wallet::Wallet;
use lazy_static::lazy_static;
use std::sync::Mutex as StdMutex;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Instant};

lazy_static! {
    static ref DB: Arc<Mutex<Option<SqliteLocalStore>>> = Arc::new(Mutex::new(None));
    static ref RUNTIME: Arc<StdMutex<Runtime>> = Arc::new(StdMutex::new(
        Runtime::new().expect("Failed to create runtime")
    ));
    static ref HTTPCLIENT: Arc<Mutex<Option<HttpClient>>> = Arc::new(Mutex::new(None));
}

macro_rules! lock_runtime {
    () => {
        match RUNTIME.lock() {
            Ok(lock) => lock,
            Err(err) => {
                let err: anyhow::Error =
                    anyhow::anyhow!("Failed to lock the runtime mutex: {}", err);
                return Err(err.into());
            }
        }
    };
}

// FIXME only call once at startup
pub fn init_db() -> anyhow::Result<u8> {
    let rt = lock_runtime!();

    let new_localstore = rt.block_on(async {
        SqliteLocalStore::with_path("../data/wallet/cashurs_wallet.db".to_string()) // FIXME make configurable
            .await
            .map_err(anyhow::Error::from)
    })?;

    rt.block_on(async {
        let mut db = DB.lock().await;
        new_localstore.migrate().await;
        *db = Some(new_localstore);

        let mut cl = HTTPCLIENT.lock().await;
        let client = HttpClient::new("http://127.0.0.1:3338".to_string()); // FIXME make configurable
        *cl = Some(client);
    });

    drop(rt);
    Ok(1)
}

pub fn get_balance() -> anyhow::Result<u64> {
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        let db = DB.lock().await;

        let total = match db.as_ref() {
            Some(db) => db.get_proofs().await?.total_amount(),
            None => 0,
        };

        Ok(total)
    });
    drop(rt);
    result
}

fn _create_local_wallet() -> anyhow::Result<Wallet> {
    let mint_url = "http://127.0.0.1:3338".to_string(); // FIXME redundant
    let rt = lock_runtime!();

    let result = rt.block_on(async move {
        let client = HTTPCLIENT.lock().await;
        let client = client.as_ref().expect("HTTPClient not set");

        let keys = client.get_mint_keys().await.map_err(anyhow::Error::from)?;

        let keysets = client
            .get_mint_keysets()
            .await
            .map_err(anyhow::Error::from)?;

        let localstore = DB.lock().await;
        let localstore = localstore.as_ref().expect("DB not set");

        Ok(Wallet::new(
            Box::new(client.to_owned()), // FIXME use borrow
            keys,
            keysets,
            Box::new(localstore.to_owned()),
            mint_url,
        ))
    });
    drop(rt);
    result
}

pub fn mint_tokens(amount: u64, hash: String) -> anyhow::Result<u64> {
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        loop {
            sleep_until(Instant::now() + Duration::from_millis(1_000)).await;
            let mint_result = wallet.mint_tokens(amount, hash.clone()).await;

            match mint_result {
                Ok(value) => {
                    return value.total_amount();
                }
                Err(cashurs_wallet::error::CashuWalletError::InvoiceNotPaidYet(_, _)) => {
                    continue;
                }
                Err(e) => {
                    println!("General Error: {}", e);
                    return 0;
                }
            }
        }
    }); // FIXME return error

    drop(rt);
    Ok(result)
}

pub fn get_mint_payment_request(amount: u64) -> anyhow::Result<FlutterPaymentRequest> {
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        wallet
            .get_mint_payment_request(amount)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(result.into())
}

#[derive(Clone)]
pub struct FlutterPaymentRequest {
    pub pr: String,
    pub hash: String,
}

impl From<PaymentRequest> for FlutterPaymentRequest {
    fn from(value: PaymentRequest) -> Self {
        Self {
            pr: value.pr,
            hash: value.hash,
        }
    }
}

pub fn pay_invoice(invoice: String) -> anyhow::Result<bool> {
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        wallet
            .pay_invoice(invoice)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(result.paid)
}

pub fn import_token(token: String) -> anyhow::Result<u64> {
    let de = TokenV3::deserialize(token).map_err(anyhow::Error::from)?;
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    rt.block_on(async {
        wallet
            .receive_tokens(&de)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(de.total_amount())
}
