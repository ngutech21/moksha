use async_trait::async_trait;
use moksha_core::proof::Proofs;

use crate::error::MokshaWalletError;

#[cfg(not(any(target_arch = "wasm32", target_os = "espidf")))]
pub mod sqlite;

#[cfg(any(target_arch = "wasm32", target_os = "espidf"))]
pub mod rexie;

#[derive(Debug, Clone)]
pub struct WalletKeyset {
    pub id: String,
    pub mint_url: String,
}

#[cfg(not(any(target_arch = "wasm32", target_os = "espidf")))]
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

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;
}

#[cfg(any(target_arch = "wasm32", target_os = "espidf"))]
pub struct RexieTransaction {}

#[cfg(any(target_arch = "wasm32", target_os = "espidf"))]
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

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;
}

#[cfg(target_os = "espidf")]
pub trait LocalStore {
    fn begin_tx(&self) -> Result<RexieTransaction, MokshaWalletError> {
        Ok(RexieTransaction {})
    }

    fn delete_proofs(
        &self,
        tx: &mut RexieTransaction,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    fn add_proofs(
        &self,
        tx: &mut RexieTransaction,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError>;
    fn get_proofs(&self, tx: &mut RexieTransaction) -> Result<Proofs, MokshaWalletError>;

    fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;
}
