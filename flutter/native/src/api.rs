use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lazy_static::lazy_static;
use lightning_invoice::Invoice;
use moksha_core::model::{
    BlindedMessage, CheckFeesResponse, Keysets, PaymentRequest, PostMeltResponse, PostMintResponse,
    PostSplitResponse, Proofs, Token, TokenV3,
};
use moksha_fedimint::FedimintWallet;
use moksha_wallet::error::MokshaWalletError;
use moksha_wallet::localstore::{LocalStore, WalletKeyset};

use moksha_wallet::config_path;

use moksha_wallet::wallet::{Wallet, WalletBuilder};
use secp256k1::PublicKey;
use std::str::FromStr;
use std::sync::Mutex as StdMutex;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex;
use url::Url;

lazy_static! {
    static ref RUNTIME: Arc<StdMutex<Runtime>> = Arc::new(StdMutex::new(
        Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("Failed to create runtime")
    ));
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

#[cfg(not(target_arch = "wasm32"))]
fn create_localstore_desktop() -> anyhow::Result<moksha_wallet::sqlx_localstore::SqliteLocalStore> {
    let rt = lock_runtime!();

    let db_path = config_path::db_path();

    #[cfg(not(target_arch = "wasm32"))]
    let lc = rt.block_on(async {
        moksha_wallet::sqlx_localstore::SqliteLocalStore::with_path(db_path.clone())
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(lc)
}

pub fn init_cashu() -> anyhow::Result<String> {
    let rt = lock_runtime!();

    let db_path = config_path::db_path();

    #[cfg(not(target_arch = "wasm32"))]
    rt.block_on(async {
        let new_localstore =
            moksha_wallet::sqlx_localstore::SqliteLocalStore::with_path(db_path.clone())
                .await
                .map_err(anyhow::Error::from)
                .unwrap(); // FIXME
        new_localstore.migrate().await;
    });

    drop(rt);
    Ok(db_path)
}

pub fn get_cashu_balance() -> anyhow::Result<u64> {
    let rt = lock_runtime!();

    #[cfg(not(target_arch = "wasm32"))]
    let result = rt.block_on(async {
        let db_path = config_path::db_path();
        let db = moksha_wallet::sqlx_localstore::SqliteLocalStore::with_path(db_path.clone())
            .await
            .map_err(anyhow::Error::from)
            .unwrap();

        Ok(db.get_proofs().await?.total_amount())
    });
    drop(rt);

    #[cfg(not(target_arch = "wasm32"))]
    return result;

    #[cfg(target_arch = "wasm32")]
    return Ok(0);
}

use moksha_wallet::client::Client;

fn _create_local_wallet() -> anyhow::Result<Wallet<impl Client, impl LocalStore>> {
    let rt = lock_runtime!();

    #[cfg(not(target_arch = "wasm32"))]
    let result = rt.block_on(async move {
        let client = moksha_wallet::reqwest_client::HttpClient::new();
        let localstore =
            moksha_wallet::sqlx_localstore::SqliteLocalStore::with_path("db_path".to_owned())
                .await
                .unwrap();
        let mint_url = reqwest::Url::parse("http://127.0.0.1:3338").expect("invalid url"); // FIXME redundant

        Ok(WalletBuilder::default()
            .with_client(client)
            .with_localstore(localstore)
            .with_mint_url(mint_url)
            .build()
            .await?)
    });

    #[cfg(target_arch = "wasm32")]
    let result = rt.block_on(async move {
        let client = MockClient::default();
        let localstore = MockLocalStore::default();

        Ok(WalletBuilder::default()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?)
    });
    drop(rt);

    result
}

#[derive(Clone)]
struct MockLocalStore {
    tokens: TokenV3,
}

impl MockLocalStore {
    fn with_tokens(tokens: TokenV3) -> Self {
        Self { tokens }
    }
}

impl Default for MockLocalStore {
    fn default() -> Self {
        Self {
            tokens: TokenV3::new(Token {
                mint: Some(Url::parse("http://127.0.0.1:3338").expect("invalid url")),
                proofs: Proofs::empty(),
            }),
        }
    }
}

#[async_trait]
impl LocalStore for MockLocalStore {
    async fn migrate(&self) {}

    async fn add_proofs(&self, _: &Proofs) -> Result<(), moksha_wallet::error::MokshaWalletError> {
        Ok(())
    }

    async fn get_proofs(
        &self,
    ) -> Result<moksha_core::model::Proofs, moksha_wallet::error::MokshaWalletError> {
        Ok(self.tokens.clone().proofs())
    }

    async fn delete_proofs(
        &self,
        _proofs: &Proofs,
    ) -> Result<(), moksha_wallet::error::MokshaWalletError> {
        Ok(())
    }

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        unimplemented!()
    }

    async fn add_keyset(&self, _keyset: &WalletKeyset) -> Result<(), MokshaWalletError> {
        unimplemented!()
    }
}

#[derive(Clone, Default)]
struct MockClient {
    split_response: PostSplitResponse,
    post_mint_response: PostMintResponse,
    post_melt_response: PostMeltResponse,
}

impl MockClient {
    fn with_split_response(split_response: PostSplitResponse) -> Self {
        Self {
            split_response,
            ..Default::default()
        }
    }

    fn with_mint_response(post_mint_response: PostMintResponse) -> Self {
        Self {
            post_mint_response,
            ..Default::default()
        }
    }

    fn with_melt_response(post_melt_response: PostMeltResponse) -> Self {
        Self {
            post_melt_response,
            split_response: PostSplitResponse::with_fst_and_snd(vec![], vec![]),
            ..Default::default()
        }
    }
}

#[async_trait]
impl Client for MockClient {
    async fn post_split_tokens(
        &self,
        _mint_url: &Url,
        _amount: u64,
        _proofs: Proofs,
        _output: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError> {
        Ok(self.split_response.clone())
    }

    async fn post_mint_payment_request(
        &self,
        _mint_url: &Url,
        _hash: String,
        _blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, MokshaWalletError> {
        Ok(self.post_mint_response.clone())
    }

    async fn post_melt_tokens(
        &self,
        _mint_url: &Url,
        _proofs: Proofs,
        _pr: String,
        _outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltResponse, MokshaWalletError> {
        Ok(self.post_melt_response.clone())
    }

    async fn post_checkfees(
        &self,
        _mint_url: &Url,
        _pr: String,
    ) -> Result<CheckFeesResponse, MokshaWalletError> {
        Ok(CheckFeesResponse { fee: 0 })
    }

    async fn get_mint_keys(
        &self,
        _mint_url: &Url,
    ) -> Result<HashMap<u64, PublicKey>, MokshaWalletError> {
        unimplemented!()
    }

    async fn get_mint_keysets(&self, _mint_url: &Url) -> Result<Keysets, MokshaWalletError> {
        unimplemented!()
    }

    async fn get_mint_payment_request(
        &self,
        _mint_url: &Url,
        _amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError> {
        unimplemented!()
    }
}

pub fn cashu_mint_tokens(amount: u64, hash: String) -> anyhow::Result<u64> {
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        for _ in 0..30 {
            tokio::time::sleep_until(tokio::time::Instant::now() + Duration::from_millis(1_000))
                .await;
            let mint_result = wallet.mint_tokens(amount.into(), hash.clone()).await;

            match mint_result {
                Ok(value) => {
                    return Ok(value.total_amount());
                }
                Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(_, _)) => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(
            amount,
            "Invoice not paid yet".to_string(),
        ))
    });

    drop(rt);
    result.map_err(anyhow::Error::from)
}

pub fn get_cashu_mint_payment_request(amount: u64) -> anyhow::Result<FlutterPaymentRequest> {
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

pub fn decode_invoice(invoice: String) -> anyhow::Result<FlutterInvoice> {
    let invoice = Invoice::from_str(&invoice).map_err(anyhow::Error::from)?;
    Ok(invoice.into())
}

#[derive(Debug, Clone)]
pub struct FlutterInvoice {
    pub pr: String,
    pub amount_sats: u64,
    pub expiry_time: u64,
}

impl From<Invoice> for FlutterInvoice {
    fn from(invoice: Invoice) -> Self {
        Self {
            pr: invoice.to_string(),
            amount_sats: match invoice.amount_milli_satoshis() {
                Some(amount) => amount / 1000,
                None => 0,
            },
            expiry_time: invoice.expiry_time().as_secs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FedimintPaymentRequest {
    pub pr: String,
    pub operation_id: String,
}

#[derive(Debug, Clone)]
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

pub fn cashu_pay_invoice(invoice: String) -> anyhow::Result<bool> {
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

fn cashu_receive_token(token: String) -> anyhow::Result<u64> {
    let deserialized_token = token.try_into().map_err(anyhow::Error::from)?;
    let wallet = _create_local_wallet().map_err(anyhow::Error::from)?;
    let rt = lock_runtime!();

    rt.block_on(async {
        wallet
            .receive_tokens(&deserialized_token)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(deserialized_token.total_amount())
}

fn fedimint_workdir() -> std::path::PathBuf {
    config_path::config_dir().join("fedimint")
}

pub fn join_federation(federation: String) -> anyhow::Result<()> {
    let rt = lock_runtime!();
    let workdir = fedimint_workdir();

    rt.block_on(async {
        FedimintWallet::connect(workdir, &federation)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(())
}

pub fn get_fedimint_payment_request(amount: u64) -> anyhow::Result<FedimintPaymentRequest> {
    let workdir = fedimint_workdir();
    let rt = lock_runtime!();

    let result = rt.block_on(async {
        let wallet = FedimintWallet::new(workdir)
            .await
            .map_err(anyhow::Error::from)?;

        wallet
            .get_mint_payment_request(amount)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);

    Ok(FedimintPaymentRequest {
        pr: result.1.to_string(),
        operation_id: result.0,
    })
}

pub fn fedimint_mint_tokens(amount: u64, operation_id: String) -> anyhow::Result<u64> {
    let workdir = fedimint_workdir();
    let rt = lock_runtime!();

    rt.block_on(async {
        let wallet = FedimintWallet::new(workdir)
            .await
            .map_err(anyhow::Error::from)?;

        wallet.mint(operation_id).await.map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(amount)
}

pub fn get_fedimint_balance() -> anyhow::Result<u64> {
    let rt = lock_runtime!();
    let workdir = fedimint_workdir();

    let result = rt.block_on(async {
        if !FedimintWallet::is_initialized(&workdir) {
            return Ok(0);
        }

        FedimintWallet::new(workdir)
            .await
            .map_err(anyhow::Error::from)?
            .balance()
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(result)
}

pub fn fedimint_pay_invoice(invoice: String) -> anyhow::Result<bool> {
    let rt = lock_runtime!();
    let workdir = fedimint_workdir();

    let result = rt.block_on(async {
        if !FedimintWallet::is_initialized(&workdir) {
            return Ok(false);
        }

        FedimintWallet::new(workdir)
            .await
            .map_err(anyhow::Error::from)?
            .pay_ln_invoice(invoice)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(result)
}

pub fn receive_token(token: String) -> anyhow::Result<u64> {
    if token.starts_with("cashu") {
        cashu_receive_token(token)
    } else {
        fedimint_receive_token(token)
    }
}

fn fedimint_receive_token(token: String) -> anyhow::Result<u64> {
    let rt = lock_runtime!();
    let workdir = fedimint_workdir();

    let result = rt.block_on(async {
        if !FedimintWallet::is_initialized(&workdir) {
            return Ok(0);
        }

        FedimintWallet::new(workdir)
            .await
            .map_err(anyhow::Error::from)?
            .receive_token(token)
            .await
            .map_err(anyhow::Error::from)
    })?;

    drop(rt);
    Ok(result)
}

fn get_btcprice_desktop() -> anyhow::Result<f64> {
    let rt = lock_runtime!();
    let result = rt.block_on(async {
        moksha_wallet::btcprice::get_btcprice()
            .await
            .map_err(anyhow::Error::from)
    })?;
    drop(rt);
    Ok(result)
}

pub fn get_btcprice() -> anyhow::Result<f64> {
    #[cfg(not(target_arch = "wasm32"))]
    return get_btcprice_desktop();

    #[cfg(target_arch = "wasm32")]
    Ok(21_000.21f64)
    // FIXME implement get_btcprice for wasm
}

// #[cfg(test)]
// mod tests {
//     use moksha_wallet::config_path;

//     use super::{get_cashu_balance, init_cashu};

//     #[test]
//     fn test_get_balance() -> anyhow::Result<()> {
//         let tmp = tempfile::tempdir().expect("Could not create tmp dir");
//         let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

//         std::env::set_var(config_path::ENV_DB_PATH, format!("{}/wallet.db", tmp_dir));
//         let _ = init_cashu()?;
//         let balance = get_cashu_balance().expect("Could not get balance");
//         assert_eq!(0, balance);
//         Ok(())
//     }
// }
