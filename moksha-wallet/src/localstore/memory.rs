use std::sync::Arc;

use async_trait::async_trait;
use moksha_core::proof::{Proof, Proofs};
use tokio::sync::Mutex;

use crate::error::MokshaWalletError;

use super::{LocalStore, WalletKeyset};

#[derive(Default, Debug, Clone)]
pub struct MemoryLocalStore {
    proofs: Arc<Mutex<Vec<Proof>>>,
}

#[async_trait(?Send)]
impl LocalStore for MemoryLocalStore {
    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError> {
        for proof in proofs.proofs() {
            self.proofs.lock().await.push(proof.clone());
        }
        Ok(())
    }

    async fn get_proofs(&self) -> Result<moksha_core::proof::Proofs, MokshaWalletError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use moksha_core::proof::{Proof, Proofs};
    use serde_json::json;

    #[tokio::test]
    async fn test_add_proofs() -> anyhow::Result<()> {
        let local_store = MemoryLocalStore::default();

        let proof1 = serde_json::from_value::<Proof>(json!(
            {
                  "id": "DSAl9nvvyfva",
                  "amount": 2,
                  "secret": "EhpennC9qB3iFlW8FZ_pZw",
                  "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
            }
        ))?;

        let proof2 = serde_json::from_value::<Proof>(json!(
            {
                  "id": "DSAl9nvvyfva",
                  "amount": 8,
                  "secret": "TmS6Cv0YT5PU_5ATVKnukw",
                  "C": "02ac910bef28cbe5d7325415d5c263026f15f9b967a079ca9779ab6e5c2db133a7"
                }
        ))?;

        let proofs = Proofs::new(vec![proof1, proof2]);
        assert_eq!(local_store.get_proofs().await?.len(), 0);
        local_store.add_proofs(&proofs).await?;
        let stored_proofs = local_store.get_proofs().await?;
        assert_eq!(stored_proofs.proofs().len(), 2);
        assert_eq!(stored_proofs.total_amount(), 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_proofs() -> anyhow::Result<()> {
        let local_store = MemoryLocalStore::default();

        let proof1 = serde_json::from_value::<Proof>(json!(
            {
                  "id": "DSAl9nvvyfva",
                  "amount": 2,
                  "secret": "EhpennC9qB3iFlW8FZ_pZw",
                  "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
            }
        ))?;

        let proof2 = serde_json::from_value::<Proof>(json!(
            {
                  "id": "DSAl9nvvyfva",
                  "amount": 8,
                  "secret": "TmS6Cv0YT5PU_5ATVKnukw",
                  "C": "02ac910bef28cbe5d7325415d5c263026f15f9b967a079ca9779ab6e5c2db133a7"
                }
        ))?;

        let proof3 = serde_json::from_value::<Proof>(json!(
        {
                "amount": 64,
                "secret": "sYYrrhUD3IwJzGFCGsUqqXXa",
                "C": "0359760ad29ae24cd8535d83d9dcf09b585d36e0649235354aa7001e60206b3a66",
                "id": "paFbO142_sui"
        }
            ))?;

        let proofs = Proofs::new(vec![proof1, proof2, proof3.clone()]);
        assert_eq!(local_store.get_proofs().await?.len(), 0);
        local_store.add_proofs(&proofs).await?;
        let stored_proofs = local_store.get_proofs().await?;
        assert_eq!(stored_proofs.proofs().len(), 3);
        assert_eq!(stored_proofs.total_amount(), 74);

        local_store
            .delete_proofs(&Proofs::new(vec![proof3]))
            .await?;
        let proofs_after_delete = local_store.get_proofs().await?;
        assert_eq!(proofs_after_delete.proofs().len(), 2);
        assert_eq!(proofs_after_delete.total_amount(), 10);

        Ok(())
    }
}
