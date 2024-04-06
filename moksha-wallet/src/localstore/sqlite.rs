use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::keyset::KeysetId;
use moksha_core::proof::{Proof, Proofs};
use secp256k1::PublicKey;
use url::Url;

use crate::error::MokshaWalletError;
use crate::localstore::{LocalStore, WalletKeyset};

use sqlx::sqlite::SqliteError;


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
            .collect::<Vec<_>>();

        let placeholders: Vec<String> = (1..=proof_secrets.len())
            .map(|i| format!("?{}", i))
            .collect();
        
        let sql = format!(
            "DELETE FROM proofs WHERE secret IN ({})",
            placeholders.join(",")
        );
        let mut query = sqlx::query(&sql);
        for secret in &proof_secrets {
            query = query.bind(secret);
        }
        query.execute(&mut **tx).await?;

        Ok(())
    }

    async fn add_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaWalletError> {
        for proof in proofs.proofs() {
            let c = proof.c.to_string();
            let amount = proof.amount as i64;
            sqlx::query!(
                "INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);",
            proof.keyset_id,amount, c, proof.secret )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn get_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MokshaWalletError> {
        let rows = sqlx::query!("SELECT keyset_id, amount, C, secret FROM proofs;")
            .fetch_all(&mut **tx)
            .await?;

        // FIXME read time_created
        Ok(rows
            .into_iter()
            .map(|row| Proof{
                    keyset_id: row.keyset_id,
                    amount: row.amount as u64,
                    c: row.C.parse().expect("Invalid Pubkey"),
                    secret: row.secret,
                    script: None,
            })
            .collect::<Vec<Proof>>()
            .into())
    }

    async fn upsert_keyset(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError> {
        let keyset_id = keyset.keyset_id.to_string();
        let mint_url = keyset.mint_url.as_str();
        let currency_unit = keyset.currency_unit.to_string();
        let last_index = keyset.last_index as i64;
        let public_keys = serde_json::to_string(&keyset.public_keys)?;
        sqlx::query!(
            r#"INSERT INTO keysets (keyset_id, mint_url, currency_unit, last_index, public_keys, active) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(keyset_id, mint_url) DO UPDATE SET currency_unit = $3, public_keys = $5, active = $6;
            "#,keyset_id, mint_url, currency_unit, last_index, public_keys, keyset.active)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn get_keysets(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        let rows = sqlx::query!("SELECT id, mint_url, keyset_id, currency_unit, active, last_index, public_keys FROM keysets;")
            .fetch_all(&mut **tx)
            .await?;
 
        Ok(rows
            .iter()
            .map(|row| {
                let id: i64 = row.id;
                let mint_url: Url = Url::parse(&row.mint_url).expect("invalid URL in localstore");
                let keyset_id: KeysetId =
                    KeysetId::new(&row.keyset_id).expect("invalid keyset_id in localstore");
                let currency_unit: String = row.currency_unit.clone();
                let active: bool = row.active;
                let last_index: i64 = row.last_index;
                let public_keys: String = row.public_keys.clone();
                let public_keys: HashMap<u64, PublicKey> =
                    serde_json::from_str(&public_keys).expect("invalid json in localstore");
                Ok(WalletKeyset {
                    id: Some(id as u64),
                    mint_url,
                    keyset_id,
                    currency_unit: currency_unit.into(),
                    active,
                    last_index: last_index as u64,
                    public_keys,
                })
            })
            .collect::<Result<Vec<WalletKeyset>, SqliteError>>()?)
    }

    async fn update_keyset_last_index(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MokshaWalletError> {
        let id = match keyset.id {
            None => return Err(MokshaWalletError::IdNotSet),
            Some(id) => id as i64,
        };
        let last_index = keyset.last_index as i64;

        sqlx::query!("UPDATE keysets SET last_index = $1 WHERE id = $2;", last_index, id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn add_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        seed_words: &str,
    ) -> Result<(), MokshaWalletError> {
        sqlx::query!("INSERT INTO seed (seed_words) VALUES ($1);", seed_words)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn get_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Option<String>, MokshaWalletError> {
        let row = sqlx::query!("SELECT seed_words FROM seed;")
            .fetch_all(&mut **tx)
            .await?;
        match row.len() {
            0 => Ok(None),
            1 => Ok(Some(row[0].seed_words.clone())),
            _ => Err(MokshaWalletError::MultipleSeeds),
        }
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
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(5))
            .idle_timeout(std::time::Duration::from_secs(5))
            .connect(connection_string)
            .await?;

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

    #[tokio::test]
    async fn test_delete_multiple_proofs() -> anyhow::Result<()> {
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
        let proof_8 = proofs.get(1).expect("Proof is empty").to_owned();

        let delete_proofs = vec![proof_4, proof_8].into();
        localstore.delete_proofs(&mut tx, &delete_proofs).await?;

        let result_tokens = localstore.get_proofs(&mut tx).await?;
        assert_eq!(48, result_tokens.total_amount());
        tx.commit().await?;
        Ok(())
    }
}
