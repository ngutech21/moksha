use async_trait::async_trait;
use cashurs_core::model::{Proof, Proofs};
use sqlx::{sqlite::SqliteError, SqlitePool};

use crate::error::CashuWalletError;

use dyn_clone::DynClone;
use sqlx::Row;

#[async_trait]
pub trait LocalStore: DynClone {
    async fn delete_proofs(&self, proofs: &Proofs) -> Result<(), CashuWalletError>;
    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), CashuWalletError>;
    async fn get_proofs(&self) -> Result<Proofs, CashuWalletError>;
    async fn migrate(&self);
}

#[derive(Clone, Debug)]
pub struct SqliteLocalStore {
    pool: sqlx::SqlitePool,
}

#[async_trait]
impl LocalStore for SqliteLocalStore {
    async fn migrate(&self) {
        sqlx::migrate!("../wallet/migrations")
            .run(&self.pool)
            .await
            .expect("Could not run migrations");
    }

    async fn delete_proofs(&self, proofs: &Proofs) -> Result<(), CashuWalletError> {
        let proof_secrets = proofs
            .proofs()
            .iter()
            .map(|p| p.secret.to_owned())
            .collect::<Vec<_>>()
            .join(", ");

        sqlx::query("DELETE FROM proofs WHERE secret in (?);")
            .bind(proof_secrets)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), CashuWalletError> {
        let tx = self.start_transaction().await?;
        for proof in proofs.proofs() {
            sqlx::query(
                r#"INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);
                "#,
            )
            .bind(proof.id)
            .bind(proof.amount as i64) // FIXME use u64
            .bind(proof.c.to_string())
            .bind(proof.secret)
            .execute(&self.pool)
            .await?;
        }
        self.commit_transaction(tx).await?;
        Ok(())
    }

    async fn get_proofs(&self) -> Result<Proofs, CashuWalletError> {
        let rows = sqlx::query("SELECT * FROM proofs;")
            .fetch_all(&self.pool)
            .await?;

        let result = rows
            .iter()
            .map(|row| {
                let id = row.get(0);
                let amount: i64 = row.get(1);
                let c: String = row.get(2);
                let secret: String = row.get(3);
                let _time_created: String = row.get(4); // TODO use time_created
                Ok(Proof {
                    id,
                    amount: amount as u64,
                    c: c.parse().unwrap(),
                    secret,
                    script: None,
                })
            })
            .collect::<Result<Vec<Proof>, SqliteError>>()
            .map(Proofs::from);

        Ok(result.unwrap()) // FIXME handle error
    }
}

impl SqliteLocalStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn with_path(absolute_path: String) -> Result<Self, CashuWalletError> {
        let pool = sqlx::SqlitePool::connect(format!("sqlite://{absolute_path}?mode=rwc").as_str())
            .await?;
        Ok(Self { pool })
    }

    pub async fn start_transaction(
        &self,
    ) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn commit_transaction(
        &self,
        transaction: sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<(), sqlx::Error> {
        transaction.commit().await
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, vec};

    use cashurs_core::model::{Proofs, TokenV3};

    use super::SqliteLocalStore;
    use crate::localstore::LocalStore;

    #[tokio::test]
    async fn test_sqlite() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let db = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db")).await?;
        db.migrate().await;

        let tx = db.start_transaction().await?;
        let tokens = read_fixture("token_60.cashu")?;
        let store: Arc<dyn LocalStore> = Arc::new(db.clone());
        store.add_proofs(&tokens.proofs()).await?;
        db.commit_transaction(tx).await?;

        let loaded_proofs = store.get_proofs().await?;
        assert_eq!(tokens.proofs(), loaded_proofs);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_tokens() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let localstore: Arc<dyn LocalStore> =
            Arc::new(SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db")).await?);
        localstore.migrate().await;

        let tokens = read_fixture("token_60.cashu")?;
        localstore.add_proofs(&tokens.proofs()).await?;

        let loaded_tokens = localstore.get_proofs().await?;

        assert_eq!(tokens.proofs(), loaded_tokens);

        let binding = tokens.tokens.get(0).unwrap().proofs.proofs();
        let proof_4 = binding.get(0).unwrap().to_owned();
        print!("first {:?}", proof_4);

        localstore
            .delete_proofs(&Proofs::from(vec![proof_4]))
            .await?;

        let result_tokens = localstore.get_proofs().await?;
        assert_eq!(56, result_tokens.total_amount());

        Ok(())
    }

    fn read_fixture(name: &str) -> anyhow::Result<TokenV3> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
        Ok(TokenV3::deserialize(raw_token.trim().to_string())?)
    }
}
