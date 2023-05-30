use std::sync::Arc;

use async_trait::async_trait;
use cashurs_core::model::{Proof, Proofs, Token, Tokens};
use rocksdb::DB;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{sqlite::SqliteError, SqlitePool};

use crate::error::CashuWalletError;

use dyn_clone::DynClone;
use sqlx::Row;

pub trait LocalStore: DynClone {
    fn delete_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError>;
    fn add_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError>;
    fn get_tokens(&self) -> Result<Tokens, CashuWalletError>;
}

#[async_trait]
pub trait LocalStore2: DynClone {
    async fn delete_proofs(&self, proofs: Proofs) -> Result<(), CashuWalletError>;
    async fn add_proofs(&self, proofs: Proofs) -> Result<(), CashuWalletError>;
    async fn get_proofs(&self) -> Result<Proofs, CashuWalletError>;
}

#[derive(Clone, Debug)]
pub struct RocksDBLocalStore {
    db: Arc<DB>,
}

#[derive(Clone, Debug)]
pub struct SqliteLocalStore {
    pool: sqlx::SqlitePool,
}

#[async_trait]
impl LocalStore2 for SqliteLocalStore {
    async fn delete_proofs(&self, proofs: Proofs) -> Result<(), CashuWalletError> {
        let proof_secrets = proofs
            .get_proofs()
            .iter()
            .map(|p| p.secret.to_owned())
            .collect::<Vec<_>>()
            .join(", ");

        sqlx::query("DELETE FROM proofs WHERE secret in (?);")
            .bind(proof_secrets)
            .execute(&self.pool)
            .await
            .unwrap();
        Ok(())
    }

    async fn add_proofs(&self, proofs: Proofs) -> Result<(), CashuWalletError> {
        let tx = self.start_transaction().await.unwrap();
        for proof in proofs.get_proofs() {
            sqlx::query(
                r#"INSERT INTO proofs (keyset_id, amount, C, secret, time_created) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP);
                "#,
            )
            .bind(proof.id)
            .bind(proof.amount as i64) // FIXME use u64
            .bind(proof.c.to_string())
            .bind(proof.secret)
            .execute(&self.pool)
            .await.unwrap();
        }
        self.commit_transaction(tx).await.unwrap();
        Ok(())
    }

    async fn get_proofs(&self) -> Result<Proofs, CashuWalletError> {
        let rows = sqlx::query("SELECT * FROM proofs;")
            .fetch_all(&self.pool)
            .await
            .unwrap();

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

        Ok(result.unwrap())
    }
}

impl SqliteLocalStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        sqlx::migrate!("../wallet/migrations")
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

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum DbKeyPrefix {
    Tokens = 0x01,
}

impl RocksDBLocalStore {
    pub fn new(path: String) -> Self {
        Self {
            db: Arc::new(DB::open_default(path).expect("Could not open database {path}")),
        }
    }

    fn put_serialized<T: Serialize + std::fmt::Debug>(
        &self,
        key: DbKeyPrefix,
        value: &T,
    ) -> Result<(), CashuWalletError> {
        match serde_json::to_string(&value) {
            Ok(serialized) => self
                .db
                .put(vec![key as u8], serialized.into_bytes())
                .map_err(CashuWalletError::from),
            Err(err) => Err(CashuWalletError::from(err)),
        }
    }

    fn get_serialized<T: DeserializeOwned>(
        &self,
        key: DbKeyPrefix,
    ) -> Result<Option<T>, CashuWalletError> {
        let entry = self.db.get(vec![key as u8])?;
        match entry {
            Some(found) => {
                let found = String::from_utf8(found)?;
                Ok(Some(serde_json::from_str::<T>(&found)?))
            }
            None => Ok(None),
        }
    }
}

impl LocalStore for RocksDBLocalStore {
    // FIXME store Proofs in Localstore instead of Tokens
    fn add_tokens(&self, new_tokens: Tokens) -> Result<(), CashuWalletError> {
        let all_tokens = self.get_tokens();

        all_tokens.and_then(|tokens| {
            if tokens.total_amount() == 0 {
                return self.put_serialized(DbKeyPrefix::Tokens, &new_tokens);
            }

            let first_token = tokens.tokens.first().expect("Tokens is empty");
            let mint = first_token.to_owned().mint.unwrap();

            let mut proofs: Vec<Proof> = vec![];
            proofs.append(&mut tokens.get_proofs().get_proofs());
            proofs.append(&mut new_tokens.get_proofs().get_proofs());

            let new_tokens = Tokens::from((mint, Proofs::from(proofs)));

            self.put_serialized(DbKeyPrefix::Tokens, &new_tokens)
        })?;
        Ok(())
    }

    fn get_tokens(&self) -> Result<Tokens, CashuWalletError> {
        self.get_serialized(DbKeyPrefix::Tokens)
            .map(|maybe_tokens| maybe_tokens.unwrap_or_else(Tokens::empty))
    }

    fn delete_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError> {
        let all_tokens = self.get_tokens()?;

        if all_tokens.tokens.is_empty() {
            return Ok(());
        }

        let all_proofs = all_tokens.get_proofs();
        let retained_proofs = all_proofs.remove(tokens.get_proofs().get_proofs());

        let first_token = all_tokens.tokens.first().expect("Tokens is empty");
        let mint = first_token.to_owned().mint;

        self.put_serialized(
            DbKeyPrefix::Tokens,
            &Tokens::new(Token {
                mint,
                proofs: retained_proofs,
            }),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, vec};

    use cashurs_core::model::{Proofs, Tokens};

    use super::{LocalStore, SqliteLocalStore};
    use crate::localstore::LocalStore2;

    #[tokio::test]
    async fn test_sqlite() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        println!("tmp dir {:?}", tmp_dir);

        let pool = sqlx::SqlitePool::connect(
            format!("sqlite:///{tmp_dir}/test_wallet.db?mode=rwc").as_str(),
        )
        .await?;
        let db = SqliteLocalStore::new(pool);
        db.migrate().await;

        let tx = db.start_transaction().await?;
        let tokens = read_fixture("token_60.cashu")?;
        let store: Arc<dyn LocalStore2> = Arc::new(db.clone());
        store.add_proofs(tokens.get_proofs()).await?;
        db.commit_transaction(tx).await?;

        let loaded_proofs = store.get_proofs().await?;
        assert_eq!(tokens.get_proofs(), loaded_proofs);

        println!("loaded proofs {:?}", loaded_proofs);

        Ok(())
    }

    #[test]
    fn test_delete_tokens() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let localstore: Arc<dyn LocalStore> =
            Arc::new(super::RocksDBLocalStore::new(tmp_dir.to_owned()));

        let tokens = read_fixture("token_60.cashu")?;
        localstore.add_tokens(tokens.clone())?;

        let loaded_tokens = localstore.get_tokens()?;

        assert_eq!(tokens, loaded_tokens);

        let binding = tokens.tokens.get(0).unwrap().proofs.get_proofs();
        let proof_4 = binding.get(0).unwrap().to_owned();
        print!("first {:?}", proof_4);

        let tokens_delete = Tokens::from((
            "http://127.0.0.1:3338".to_string(),
            Proofs::from(vec![proof_4]),
        ));

        localstore.delete_tokens(tokens_delete)?;

        let result_tokens = localstore.get_tokens()?;
        dbg!(&result_tokens);

        assert_eq!(56, result_tokens.total_amount());

        Ok(())
    }

    fn read_fixture(name: &str) -> anyhow::Result<Tokens> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
        Ok(Tokens::deserialize(raw_token.trim().to_string())?)
    }
}
