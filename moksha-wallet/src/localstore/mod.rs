use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::{primitives::CurrencyUnit, proof::Proofs};
use secp256k1::PublicKey;
use url::Url;

use crate::error::MokshaWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;

#[cfg(target_arch = "wasm32")]
pub mod rexie;

#[derive(Debug, Clone)]
pub struct WalletKeyset {
    /// primary key
    pub id: Option<u64>,
    pub keyset_id: String,
    pub mint_url: Url,
    pub currency_unit: CurrencyUnit,
    /// last index used for deriving keys from the master key
    pub last_index: u64,
    pub public_keys: HashMap<u64, PublicKey>,
    pub active: bool,
}

impl WalletKeyset {
    pub fn new(
        keyset_id: &str,
        mint_url: &Url,
        currency_unit: &CurrencyUnit,
        last_index: u64,
        public_keys: HashMap<u64, PublicKey>,
        active: bool,
    ) -> Self {
        Self {
            id: None,
            keyset_id: keyset_id.to_owned(),
            mint_url: mint_url.to_owned(),
            currency_unit: currency_unit.clone(),
            last_index,
            public_keys,
            active,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait(?Send)]
pub trait LocalStore {
    type DB: sqlx::Database;
    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, MokshaWalletError>;
    async fn delete_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn add_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn get_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MokshaWalletError>;

    async fn get_keysets(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn upsert_keyset(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError>;

    async fn update_keyset_last_index(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError>;

    async fn add_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        seed_words: &str,
    ) -> Result<(), MokshaWalletError>;

    async fn get_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Option<String>, MokshaWalletError>;
}

#[cfg(target_arch = "wasm32")]
pub struct RexieTransaction {}

#[cfg(target_arch = "wasm32")]
impl RexieTransaction {
    pub async fn commit(&self) -> Result<(), MokshaWalletError> {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait LocalStore {
    async fn begin_tx(&self) -> Result<RexieTransaction, MokshaWalletError> {
        Ok(RexieTransaction {})
    }

    async fn delete_proofs(
        &self,
        tx: &mut RexieTransaction,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn add_proofs(
        &self,
        tx: &mut RexieTransaction,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn get_proofs(&self, tx: &mut RexieTransaction) -> Result<Proofs, MokshaWalletError>;

    async fn get_keysets(
        &self,
        _tx: &mut RexieTransaction,
    ) -> Result<Vec<WalletKeyset>, MokshaWalletError>;

    async fn upsert_keyset(
        &self,
        _tx: &mut RexieTransaction,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError>;

    async fn update_keyset_last_index(
        &self,
        _tx: &mut RexieTransaction,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError>;

    async fn add_seed(
        &self,
        _tx: &mut RexieTransaction,
        seed_words: &str,
    ) -> Result<(), MokshaWalletError>;

    async fn get_seed(
        &self,
        _tx: &mut RexieTransaction,
    ) -> Result<Option<String>, MokshaWalletError>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use secp256k1::PublicKey;

    fn generate_test_map() -> HashMap<u32, PublicKey> {
        let mut map = HashMap::new();
        let secp = secp256k1::Secp256k1::new();

        for i in 0..10 {
            let secret_key = secp256k1::SecretKey::new(&mut secp256k1::rand::thread_rng());
            let public_key = PublicKey::from_secret_key(&secp, &secret_key);
            map.insert(i, public_key);
        }

        map
    }

    #[test]
    fn test_() {
        //let x: HashMap<u64, PublicKey<Secp256k1>, RandomState>;
        let data = generate_test_map();
        let json = serde_json::to_string(&data).unwrap();
        println!("{:?}", json);
    }
}
