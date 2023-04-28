use std::sync::Arc;

use cashurs_core::model::Proofs;
use rocksdb::DB;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CashuMintError;

#[derive(Clone)]
pub struct Database {
    db: Arc<DB>,
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum DbKeyPrefix {
    UsedProofs = 0x01,
}

impl Database {
    pub fn new(path: String) -> Self {
        Self {
            db: Arc::new(DB::open_default(path).expect("Could not open database {path}")),
        }
    }

    fn put_serialized<T: Serialize + std::fmt::Debug>(
        &self,
        key: DbKeyPrefix,
        value: &T,
    ) -> Result<(), CashuMintError> {
        match serde_json::to_string(&value) {
            Ok(serialized) => self
                .db
                .put(vec![key as u8], serialized.into_bytes())
                .map_err(CashuMintError::from),
            Err(err) => Err(CashuMintError::from(err)),
        }
    }

    fn get_serialized<T: DeserializeOwned>(
        &self,
        key: DbKeyPrefix,
    ) -> Result<Option<T>, CashuMintError> {
        let entry = self.db.get(vec![key as u8])?;
        match entry {
            Some(found) => {
                let found = String::from_utf8(found)?;
                Ok(Some(serde_json::from_str::<T>(&found)?))
            }
            None => Ok(None),
        }
    }

    pub fn write_used_proofs(&self, proofs: Proofs) -> Result<(), CashuMintError> {
        self.put_serialized(DbKeyPrefix::UsedProofs, &proofs)
    }

    pub fn read_used_proofs(&self) -> Result<Proofs, CashuMintError> {
        self.get_serialized::<Proofs>(DbKeyPrefix::UsedProofs)
            .map(|maybe_proofs| maybe_proofs.unwrap_or_else(Proofs::empty))
    }
}

#[cfg(test)]
mod tests {
    use cashurs_core::{
        dhke,
        model::{Proof, Proofs},
    };

    #[test]
    fn test_write_proofs() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let db = super::Database::new(tmp_dir.to_owned());

        let proofs = Proofs::new(vec![Proof {
            amount: 21,
            secret: "secret".to_string(),
            c: dhke::public_key_from_hex(
                "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
            ),
            id: None,
            script: None,
        }]);

        db.write_used_proofs(proofs.clone())?;
        let new_proofs = db.read_used_proofs()?;
        assert_eq!(proofs, new_proofs);
        Ok(())
    }

    #[test]
    fn test_read_empty_proofs() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
        let db = super::Database::new(tmp_dir.to_owned());

        let new_proofs = db.read_used_proofs()?;
        assert!(new_proofs.is_empty());
        Ok(())
    }
}
