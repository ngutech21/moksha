use super::{LocalStore, WalletKeyset};
use crate::error::MokshaWalletError;
use async_trait::async_trait;
use moksha_core::proof::{Proof, Proofs};
use rexie::*;
use wasm_bindgen::JsValue;

#[derive(Clone, Default)]
pub struct RexieLocalStore;

const STORE_NAME: &str = "proofs";

impl RexieLocalStore {
    pub async fn new() -> Self {
        Self {}
    }
}

impl RexieLocalStore {
    async fn get_rexie() -> Rexie {
        Rexie::builder("moksha")
            .version(1)
            .add_object_store(ObjectStore::new(STORE_NAME))
            .build()
            .await
            .unwrap()
    }

    fn get_key(proof: &Proof) -> JsValue {
        let key = serde_json::json!({
            "key": proof.secret,
        });
        let key = serde_json::to_string(&key).unwrap();
        serde_wasm_bindgen::to_value(&key).unwrap()
    }
}

#[async_trait(?Send)]
impl LocalStore for RexieLocalStore {
    async fn add_proofs(&self, proofs: &Proofs) -> std::result::Result<(), MokshaWalletError> {
        let db = Self::get_rexie().await;

        for proof in proofs.proofs() {
            let transaction = db
                .transaction(&[STORE_NAME], rexie::TransactionMode::ReadWrite)
                .expect("db error");
            let store = transaction.store(STORE_NAME).expect("db error");
            let json = serde_json::to_string(&proof).unwrap();
            let js_value = serde_wasm_bindgen::to_value(&json).unwrap();

            store
                .add(&js_value, Some(&Self::get_key(&proof)))
                .await
                .expect("db store error");
            transaction.done().await.expect("db error");
        }

        Ok(())
    }

    async fn get_proofs(&self) -> std::result::Result<Proofs, MokshaWalletError> {
        let db = Self::get_rexie().await;
        let transaction = db
            .transaction(&[STORE_NAME], rexie::TransactionMode::ReadOnly)
            .expect("db error");
        let store = transaction.store(STORE_NAME).expect("db error");
        let all = store.get_all(None, None, None, None).await;
        match all {
            Ok(all) => {
                let mut proofs = vec![];
                for (_, proof) in all {
                    let proof: String = serde_wasm_bindgen::from_value(proof).unwrap();
                    let proof = serde_json::from_str::<Proof>(&proof).unwrap();
                    proofs.push(proof);
                }
                Ok(Proofs::new(proofs))
            }
            Err(_) => Ok(Proofs::new(vec![])),
        }
    }

    async fn delete_proofs(
        &self,
        proofs_to_delete: &Proofs,
    ) -> std::result::Result<(), MokshaWalletError> {
        let db = Self::get_rexie().await;

        for proof in proofs_to_delete.proofs() {
            let transaction = db
                .transaction(&[STORE_NAME], rexie::TransactionMode::ReadWrite)
                .expect("db error");
            let store = transaction.store(STORE_NAME).expect("db error");

            store
                .delete(&Self::get_key(&proof))
                .await
                .expect("db error");
            transaction.done().await.expect("db error");
        }

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
