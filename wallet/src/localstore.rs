use std::sync::Arc;

use cashurs_core::model::Tokens;
use rocksdb::DB;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CashuWalletError;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait LocalStore {
    fn add_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError>;
    fn get_tokens(&self) -> Result<Tokens, CashuWalletError>;
}

pub struct RocksDBLocalStore {
    db: Arc<DB>,
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
    fn add_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError> {
        self.put_serialized(DbKeyPrefix::Tokens, &tokens)
    }

    fn get_tokens(&self) -> Result<Tokens, CashuWalletError> {
        self.get_serialized(DbKeyPrefix::Tokens)
            .map(|maybe_tokens| maybe_tokens.unwrap_or_else(Tokens::empty))
    }
}
