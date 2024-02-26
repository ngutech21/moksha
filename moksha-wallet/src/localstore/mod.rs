use async_trait::async_trait;
use moksha_core::proof::Proofs;
use sqlx::Transaction;

use crate::error::MokshaWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;

#[cfg(target_arch = "wasm32")]
pub mod rexie;

#[derive(Debug, Clone)]
pub struct WalletKeyset {
    pub id: String,
    pub mint_url: String,
}

#[async_trait(?Send)]
pub trait LocalStore {
    type DB: sqlx::Database;
    async fn begin_tx(&self) -> Result<Transaction<'_, Self::DB>, MokshaWalletError>;
    async fn delete_proofs(
        &self,
        tx: &mut Transaction<'_, Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn add_proofs(
        &self,
        tx: &mut Transaction<'_, Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    async fn get_proofs(
        &self,
        tx: &mut Transaction<'_, Self::DB>,
    ) -> Result<Proofs, MokshaWalletError>;

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;
}
