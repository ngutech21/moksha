use std::sync::Arc;

use async_trait::async_trait;
use moksha_core::model::{Proof, Proofs};
use tokio::sync::Mutex;

use crate::error::MokshaWalletError;

use super::{LocalStore, WalletKeyset};

#[derive(Default, Debug, Clone)]
pub struct MemoryLocalStore {
    proofs: Arc<Mutex<Vec<Proof>>>,
}

#[async_trait(?Send)]
impl LocalStore for MemoryLocalStore {
    async fn migrate(&self) {}

    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError> {
        for proof in proofs.proofs() {
            self.proofs.lock().await.push(proof.clone());
        }
        Ok(())
    }

    async fn get_proofs(&self) -> Result<moksha_core::model::Proofs, MokshaWalletError> {
        Ok(Proofs::new(self.proofs.lock().await.clone()))
    }

    async fn delete_proofs(&self, proofs_to_delete: &Proofs) -> Result<(), MokshaWalletError> {
        for proof in proofs_to_delete.proofs() {
            self.proofs.lock().await.retain(|p| p != &proof);
        }
        Ok(())
    }

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        // FIXME todo implement
        Ok(vec![WalletKeyset {
            id: "id".to_string(),
            mint_url: "mint_url".to_string(),
        }])
    }

    async fn add_keyset(&self, _keyset: &WalletKeyset) -> Result<(), MokshaWalletError> {
        // FIXME todo implement
        Ok(())
    }
}
