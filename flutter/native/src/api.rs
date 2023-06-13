// This is the entry point of your Rust library.
// When adding new code to your project, note that only items used
// here will be transformed to their Dart equivalents.

use std::io::Error;
use std::sync::Arc;

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

lazy_static! {
    static ref DB: Arc<Mutex<Option<SqliteLocalStore>>> = Arc::new(Mutex::new(None));
    static ref RUNTIME: Arc<StdMutex<Runtime>> = Arc::new(StdMutex::new(Runtime::new().unwrap()));
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
        SqliteLocalStore::with_path("../data/wallet/cashurs_wallet.db".to_string())
            .await
            .map_err(anyhow::Error::from)
            .unwrap() // FIXME
    });

    rt.block_on(async {
        let mut db = DB.lock().await;
        *db = Some(new_localstore);

        let mut cl = HTTPCLIENT.lock().await;
        let client = HttpClient::new("http://127.0.0.1:3338".to_string());
        *cl = Some(client);
    });

    drop(rt);
    Ok(1)
}

pub fn get_balance() -> anyhow::Result<u64> {
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        let db = DB.lock().await;

        //let db = db.as_ref().ok_or_else(|| Error("DB not set".to_string()))?;

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
        let client = client.as_ref().unwrap();

        let keys = client.get_mint_keys().await.map_err(anyhow::Error::from)?;

        let keysets = client
            .get_mint_keysets()
            .await
            .map_err(anyhow::Error::from)?;
        println!("get_balance() localstore");

        let localstore = DB.lock().await;
        let localstore = localstore.as_ref().unwrap();

        //   localstore.migrate().await; // FIXME

        Ok(Wallet::new(
            Box::new(client.to_owned()),
            keys,
            keysets,
            Box::new(localstore.to_owned()),
            mint_url,
        ))
    });
    drop(rt);
    result
}

pub fn import_token(token: String) -> anyhow::Result<u64> {
    let de = TokenV3::deserialize(token).map_err(anyhow::Error::from)?;
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;

    let rt = lock_runtime!();

    rt.block_on(async {
        wallet
            .receive_tokens(&de)
            .await
            .map_err(anyhow::Error::from); // FIXME
    });

    Ok(de.total_amount())
}
