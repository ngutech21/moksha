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
    ) -> Result<(), String> {
        match serde_json::to_string(&value) {
            Ok(serialized) => self
                .db
                .put(vec![key as u8], serialized.into_bytes())
                .map_err(|err| format!("Failed to put in db:{:?}", err)),
            Err(err) => Err(format!(
                "Failed to serialize to String. T: {:?}, err: {:?}",
                value, err
            )),
        }
    }

    fn get_serialized<T: DeserializeOwned>(&self, key: DbKeyPrefix) -> Result<Option<T>, String> {
        match self.db.get(vec![key as u8]) {
            Ok(opt) => match opt {
                Some(found) => match String::from_utf8(found) {
                    Ok(s) => match serde_json::from_str::<T>(&s) {
                        Ok(t) => Ok(Some(t)),
                        Err(err) => Err(format!("Failed to deserialize: {:?}", err)),
                    },
                    Err(err) => Err(format!("Failed to convert to String: {:?}", err)),
                },
                None => Ok(None),
            },
            Err(err) => Err(format!("Failed to get DB: {:?}", err)),
        }
    }

    pub fn write_used_proofs(&self, proofs: Proofs) {
        match self.put_serialized(DbKeyPrefix::UsedProofs, &proofs) {
            Ok(_) => (),
            Err(err) => println!("Failed to write proofs to db: {:?}", err),
        }
    }

    pub fn read_used_proofs(&self) -> Result<Proofs, CashuMintError> {
        match self.get_serialized::<Proofs>(DbKeyPrefix::UsedProofs) {
            Ok(opt) => match opt {
                Some(proofs) => Ok(proofs),
                None => Err(CashuMintError::Db("No proofs found".to_string())),
            },
            Err(err) => Err(CashuMintError::Db(format!(
                "Failed to read proofs from db: {:?}",
                err
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use cashurs_core::{dhke, model::Proof};
    use tempdir::TempDir;

    #[test]
    fn test_database() -> anyhow::Result<()> {
        let tmp_dir = TempDir::new("db-test")?
            .path()
            .to_str()
            .unwrap()
            .to_string();
        let db = super::Database::new(tmp_dir);

        let proofs = vec![Proof {
            amount: 21,
            secret: "secret".to_string(),
            c: dhke::public_key_from_hex(
                "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
            ),
            id: None,
            script: None,
        }];

        db.write_used_proofs(proofs.clone());
        let new_proofs = db.read_used_proofs()?;
        assert_eq!(proofs, new_proofs);
        Ok(())
    }
}
