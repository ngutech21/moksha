use async_trait::async_trait;
use moksha_core::model::Proofs;

use crate::error::MokshaWalletError;

#[derive(Debug, Clone)]
pub struct WalletKeyset {
    pub id: String,
    pub mint_url: String,
}

#[async_trait]
pub trait LocalStore {
    async fn delete_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError>;
    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError>;
    async fn get_proofs(&self) -> Result<Proofs, MokshaWalletError>;

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;

    async fn migrate(&self);
}
