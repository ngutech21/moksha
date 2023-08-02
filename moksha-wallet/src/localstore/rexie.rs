use super::{LocalStore, WalletKeyset};
use crate::error::MokshaWalletError;
use async_trait::async_trait;
use moksha_core::model::{Proof, Proofs};
use rexie::*;

#[derive(Clone)]
pub struct RexieLocalStore {
    db: std::sync::Arc<tokio::sync::Mutex<Rexie>>,
}

impl RexieLocalStore {
    pub async fn new() -> Self {
        let rexie = Rexie::builder("moksha")
            .version(1)
            .add_object_store(
                ObjectStore::new("proofs")
                    // Set the key path to `id`
                    .key_path("secret")
                    // Add an index named `email` with the key path `email` with unique enabled
                    .add_index(Index::new("secret", "secret").unique(true)),
            )
            // Build the database
            .build()
            .await
            .unwrap();

        Self {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(rexie)),
        }
    }
}

#[async_trait(?Send)]
impl LocalStore for RexieLocalStore {
    async fn migrate(&self) {}

    async fn add_proofs(&self, proofs: &Proofs) -> std::result::Result<(), MokshaWalletError> {
        // for proof in proofs.proofs() {
        //     self.proofs.lock().await.push(proof.clone());
        // }
        Ok(())
    }

    async fn get_proofs(&self) -> std::result::Result<Proofs, MokshaWalletError> {
        //Ok(Proofs::new(self.proofs.lock().await.clone()))
        todo!()
    }

    async fn delete_proofs(
        &self,
        proofs_to_delete: &Proofs,
    ) -> std::result::Result<(), MokshaWalletError> {
        // for proof in proofs_to_delete.proofs() {
        //     self.proofs.lock().await.retain(|p| p != &proof);
        // }
        Ok(())
    }

    async fn get_keysets(&self) -> std::result::Result<Vec<WalletKeyset>, MokshaWalletError> {
        // FIXME todo implement
        Ok(vec![WalletKeyset {
            id: "id".to_string(),
            mint_url: "mint_url".to_string(),
        }])
    }

    async fn add_keyset(
        &self,
        _keyset: &WalletKeyset,
    ) -> std::result::Result<(), MokshaWalletError> {
        // FIXME todo implement
        Ok(())
    }
}
