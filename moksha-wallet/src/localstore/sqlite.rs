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

    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, MokshaWalletError> {
        Ok(self.pool.begin().await?)
    }

    async fn delete_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
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
            .await?;
        Ok(())
    }

    async fn add_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError> {
        for proof in proofs.proofs() {
            sqlx::query(
                r#"INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);
                "#,
            )
            .bind(proof.keyset_id)
            .bind(proof.amount as i64)
            .bind(proof.c.to_string())
            .bind(proof.secret)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn get_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MokshaWalletError> {
        let rows = sqlx::query("SELECT * FROM proofs;")
            .fetch_all(&mut **tx)
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
                    c: c.parse().expect("Invalid Pubkey"),
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
        Self::with_connection_string(&format!("sqlite://{absolute_path}?mode=rwc")).await
    }

    pub async fn with_in_memory() -> Result<Self, MokshaWalletError> {
        Self::with_connection_string("sqlite::memory:").await
    }

    async fn with_connection_string(connection_string: &str) -> Result<Self, MokshaWalletError> {
        // creates db-file if not already exists
        let pool = sqlx::SqlitePool::connect(connection_string).await?;

        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteLocalStore;
    use crate::localstore::LocalStore;
    use moksha_core::{fixture::read_fixture, token::TokenV3};

    #[tokio::test]
    async fn test_add_proofs() -> anyhow::Result<()> {
        let db = SqliteLocalStore::with_in_memory().await?;
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
    async fn test_delete_proofs() -> anyhow::Result<()> {
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
