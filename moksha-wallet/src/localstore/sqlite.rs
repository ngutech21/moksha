use async_trait::async_trait;
use moksha_core::proof::{Proof, Proofs};

use crate::error::MokshaWalletError;
use crate::localstore::{LocalStore, WalletKeyset};

use sqlx::sqlite::SqliteError;

use sqlx::Row;

#[derive(Clone, Debug)]
pub struct SqliteLocalStore {
    pool: sqlx::SqlitePool,
}

#[async_trait(?Send)]
impl LocalStore for SqliteLocalStore {
    async fn delete_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError> {
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

    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError> {
        let tx = self.start_transaction().await?;
        for proof in proofs.proofs() {
            sqlx::query(
                r#"INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);
                "#,
            )
            .bind(proof.keyset_id)
            .bind(proof.amount as i64) // FIXME use u64
            .bind(proof.c.to_string())
            .bind(proof.secret)
            .execute(&self.pool)
            .await?;
        }
        self.commit_transaction(tx).await?;
        Ok(())
    }

    async fn get_proofs(&self) -> Result<Proofs, MokshaWalletError> {
        let rows = sqlx::query("SELECT * FROM proofs;")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|row| {
                let id = row.get(0);
                let amount: i64 = row.get(1);
                let c: String = row.get(2);
                let secret: String = row.get(3);
                let _time_created: String = row.get(4); // TODO use time_created
                Ok(Proof {
                    keyset_id: id,
                    amount: amount as u64,
                    c: c.parse().unwrap(),
                    secret,
                    script: None,
                })
            })
            .collect::<Result<Vec<Proof>, SqliteError>>()?
            .into())
    }

    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError> {
        sqlx::query(
            r#"INSERT INTO keysets (id, mint_url) VALUES ($1, $2);
            "#,
        )
        .bind(keyset.id.to_owned())
        .bind(keyset.mint_url.to_owned())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        let rows = sqlx::query("SELECT * FROM keysets;")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|row| {
                let id = row.get(0);
                let mint_url: String = row.get(1);
                Ok(WalletKeyset { id, mint_url })
            })
            .collect::<Result<Vec<WalletKeyset>, SqliteError>>()?)
    }
}

impl SqliteLocalStore {
    pub async fn with_path(absolute_path: String) -> Result<Self, MokshaWalletError> {
        let pool = sqlx::SqlitePool::connect(format!("sqlite://{absolute_path}?mode=rwc").as_str())
            .await?;
        let store = Self { pool };
        store.migrate().await;
        Ok(store)
    }

    async fn migrate(&self) {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .expect("Could not run migrations");
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
    use std::sync::Arc;

    use moksha_core::{fixture::read_fixture, token::TokenV3};

    use super::SqliteLocalStore;
    use crate::localstore::LocalStore;

    #[tokio::test]
    async fn test_sqlite() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let db = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db")).await?;
        db.migrate().await;

        let tx = db.start_transaction().await?;
        let tokens: TokenV3 = read_fixture("token_60.cashu")?
            .trim()
            .to_string()
            .try_into()?;
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

        let tokens: TokenV3 = read_fixture("token_60.cashu")?
            .trim()
            .to_string()
            .try_into()?;
        localstore.add_proofs(&tokens.proofs()).await?;

        let loaded_tokens = localstore.get_proofs().await?;

        assert_eq!(tokens.proofs(), loaded_tokens);

        let proofs = tokens
            .tokens
            .first()
            .expect("Tokens is empty")
            .proofs
            .proofs();
        let proof_4 = proofs.first().expect("Proof is empty").to_owned();
        print!("first {:?}", proof_4);

        localstore.delete_proofs(&proof_4.into()).await?;

        let result_tokens = localstore.get_proofs().await?;
        assert_eq!(56, result_tokens.total_amount());

        Ok(())
    }
}
