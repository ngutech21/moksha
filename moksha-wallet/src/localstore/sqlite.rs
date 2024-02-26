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
    type DB = sqlx::Sqlite;

    async fn begin_tx(&self) -> Result<sqlx::Transaction<'_, Self::DB>, MokshaWalletError> {
        Ok(self.pool.begin().await.unwrap())
    }

    async fn delete_proofs(
        &self,
        tx: &mut sqlx::Transaction<'_, Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError> {
        let proof_secrets = proofs
            .proofs()
            .iter()
            .map(|p| p.secret.to_owned())
            .collect::<Vec<_>>()
            .join(", ");

        sqlx::query("DELETE FROM proofs WHERE secret in (?);")
            .bind(proof_secrets)
            .execute(&mut **tx)
            .await
            .unwrap();
        Ok(())
    }

    async fn add_proofs(
        &self,
        tx: &mut sqlx::Transaction<'_, Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError> {
        for proof in proofs.proofs() {
            sqlx::query(
                r#"INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);
                "#,
            )
            .bind(proof.keyset_id)
            .bind(proof.amount as i64) // FIXME use u64
            .bind(proof.c.to_string())
            .bind(proof.secret)
            .execute(&mut **tx)
            .await.unwrap();
        }
        Ok(())
    }

    async fn get_proofs(
        &self,
        tx: &mut sqlx::Transaction<'_, Self::DB>,
    ) -> Result<Proofs, MokshaWalletError> {
        let rows = sqlx::query("SELECT * FROM proofs;")
            .fetch_all(&mut **tx)
            .await
            .unwrap();

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
            .collect::<Result<Vec<Proof>, SqliteError>>()
            .unwrap()
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
        .await
        .unwrap();
        Ok(())
    }

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        let rows = sqlx::query("SELECT * FROM keysets;")
            .fetch_all(&self.pool)
            .await
            .unwrap();

        Ok(rows
            .iter()
            .map(|row| {
                let id = row.get(0);
                let mint_url: String = row.get(1);
                Ok(WalletKeyset { id, mint_url })
            })
            .collect::<Result<Vec<WalletKeyset>, SqliteError>>()
            .unwrap())
    }
}

impl SqliteLocalStore {
    pub async fn with_path(absolute_path: String) -> Result<Self, MokshaWalletError> {
        Self::with_connection_string(&format!("sqlite://{absolute_path}?mode=rwc")).await
    }

    async fn with_connection_string(connection_string: &str) -> Result<Self, MokshaWalletError> {
        // creates db-file if not already exists
        let pool = sqlx::SqlitePool::connect(connection_string).await.unwrap();

        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .unwrap();
        let store = Self { pool };
        store.migrate().await.unwrap();
        Ok(store)
    }

    pub async fn with_in_memory() -> Result<Self, MokshaWalletError> {
        Self::with_connection_string("sqlite::memory:").await
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
        Ok(sqlx::migrate!("./migrations").run(&self.pool).await?)
    }
}

#[cfg(test)]
mod tests {

    use moksha_core::{fixture::read_fixture, token::TokenV3};

    use super::SqliteLocalStore;
    use crate::localstore::LocalStore;

    #[tokio::test]
    async fn test_sqlite() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let db = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db")).await?;
        db.migrate().await?;

        let mut tx = db.begin_tx().await?;
        let tokens: TokenV3 = read_fixture("token_60.cashu")?
            .trim()
            .to_string()
            .try_into()?;

        db.add_proofs(&mut tx, &tokens.proofs()).await?;

        let loaded_proofs = db.get_proofs(&mut tx).await?;
        assert_eq!(tokens.proofs(), loaded_proofs);
        tx.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_tokens() -> anyhow::Result<()> {
        let localstore = SqliteLocalStore::with_in_memory().await?;
        let mut tx = localstore.begin_tx().await?;

        let tokens: TokenV3 = read_fixture("token_60.cashu")?
            .trim()
            .to_string()
            .try_into()?;
        localstore.add_proofs(&mut tx, &tokens.proofs()).await?;

        let loaded_tokens = localstore.get_proofs(&mut tx).await?;

        assert_eq!(tokens.proofs(), loaded_tokens);

        let proofs = tokens
            .tokens
            .first()
            .expect("Tokens is empty")
            .proofs
            .proofs();
        let proof_4 = proofs.first().expect("Proof is empty").to_owned();
        localstore.delete_proofs(&mut tx, &proof_4.into()).await?;

        let result_tokens = localstore.get_proofs(&mut tx).await?;
        assert_eq!(56, result_tokens.total_amount());
        tx.commit().await?;
        Ok(())
    }
}
