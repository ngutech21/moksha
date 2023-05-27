use std::sync::Arc;

use cashurs_core::model::{Proof, Proofs, Token, Tokens};
use rocksdb::DB;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CashuWalletError;

use dyn_clone::DynClone;

pub trait LocalStore: DynClone {
    fn delete_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError>;
    fn add_tokens(&self, tokens: Tokens) -> Result<(), CashuWalletError>;
    fn get_tokens(&self) -> Result<Tokens, CashuWalletError>;
}

#[derive(Clone, Debug)]
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

    use super::LocalStore;

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
