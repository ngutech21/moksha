use std::{collections::HashMap, sync::Arc};

use moksha_core::model::Proofs;
use rocksdb::DB;
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::MokshaMintError, model::Invoice};
#[cfg(test)]
use mockall::automock;

#[derive(Clone)]
pub struct RocksDB {
    db: Arc<DB>,
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum DbKeyPrefix {
    UsedProofs = 0x01,
    PendingInvoices = 0x02,
}

#[cfg_attr(test, automock)]
pub trait Database {
    fn add_used_proofs(&self, proofs: &Proofs) -> Result<(), MokshaMintError>;
    fn get_used_proofs(&self) -> Result<Proofs, MokshaMintError>;

    fn get_pending_invoice(&self, key: String) -> Result<Invoice, MokshaMintError>;
    fn get_pending_invoices(&self) -> Result<HashMap<String, Invoice>, MokshaMintError>;
    fn add_pending_invoice(&self, key: String, invoice: Invoice) -> Result<(), MokshaMintError>;
    fn remove_pending_invoice(&self, key: String) -> Result<(), MokshaMintError>;
}

impl RocksDB {
    pub fn new(path: String) -> Self {
        Self {
            db: Arc::new(DB::open_default(path).expect("Could not open database {path}")),
        }
    }

    fn put_serialized<T: Serialize + std::fmt::Debug>(
        &self,
        key: DbKeyPrefix,
        value: &T,
    ) -> Result<(), MokshaMintError> {
        match serde_json::to_string(&value) {
            Ok(serialized) => self
                .db
                .put(vec![key as u8], serialized.into_bytes())
                .map_err(MokshaMintError::from),
            Err(err) => Err(MokshaMintError::from(err)),
        }
    }

    fn get_serialized<T: DeserializeOwned>(
        &self,
        key: DbKeyPrefix,
    ) -> Result<Option<T>, MokshaMintError> {
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

impl Database for RocksDB {
    fn add_used_proofs(&self, proofs: &Proofs) -> Result<(), MokshaMintError> {
        let used_proofs = self.get_used_proofs()?;

        let insert = Proofs::new(
            used_proofs
                .proofs()
                .into_iter()
                .chain(proofs.proofs())
                .collect(),
        );
        self.put_serialized(DbKeyPrefix::UsedProofs, &insert)?;

        Ok(())
    }

    fn get_used_proofs(&self) -> Result<Proofs, MokshaMintError> {
        self.get_serialized::<Proofs>(DbKeyPrefix::UsedProofs)
            .map(|maybe_proofs| maybe_proofs.unwrap_or_else(Proofs::empty))
    }

    fn get_pending_invoices(&self) -> Result<HashMap<String, Invoice>, MokshaMintError> {
        self.get_serialized::<HashMap<String, Invoice>>(DbKeyPrefix::PendingInvoices)
            .map(|maybe_proofs| maybe_proofs.unwrap_or_default())
    }

    fn get_pending_invoice(&self, key: String) -> Result<Invoice, MokshaMintError> {
        let invoices = self
            .get_serialized::<HashMap<String, Invoice>>(DbKeyPrefix::PendingInvoices)
            .map(|maybe_proofs| maybe_proofs.unwrap_or_default());
        invoices.and_then(|invoices| {
            invoices
                .get(&key)
                .cloned()
                .ok_or_else(|| MokshaMintError::InvoiceNotFound(key))
        })
    }

    fn add_pending_invoice(&self, key: String, invoice: Invoice) -> Result<(), MokshaMintError> {
        let invoices = self.get_pending_invoices();

        invoices.and_then(|mut invoices| {
            invoices.insert(key, invoice);
            self.put_serialized(DbKeyPrefix::PendingInvoices, &invoices)
        })?;

        Ok(())
    }

    fn remove_pending_invoice(&self, key: String) -> Result<(), MokshaMintError> {
        let invoices = self.get_pending_invoices();

        invoices.and_then(|mut invoices| {
            invoices.remove(key.as_str());
            self.put_serialized(DbKeyPrefix::PendingInvoices, &invoices)
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use moksha_core::{
        dhke,
        model::{Proof, Proofs},
    };

    use crate::{database::Database, model::Invoice};

    #[test]
    fn test_write_proofs() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

        let db: Arc<dyn Database> = Arc::new(super::RocksDB::new(tmp_dir.to_owned()));

        let keyset_id = "keyset_id";
        let proofs = Proofs::with_proof(Proof::new(
            21,
            "secret".to_string(),
            dhke::public_key_from_hex(
                "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
            ),
            keyset_id.to_owned(),
        ));

        db.add_used_proofs(&proofs)?;
        let new_proofs = db.get_used_proofs()?;
        assert_eq!(proofs, new_proofs);

        let proofs2 = Proofs::with_proof(Proof::new(
            42,
            "secret 2".to_string(),
            dhke::public_key_from_hex(
                "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
            ),
            keyset_id.to_owned(),
        ));

        db.add_used_proofs(&proofs2)?;
        let result_proofs = db.get_used_proofs()?;
        assert!(result_proofs.len() == 2);

        Ok(())
    }

    #[test]
    fn test_read_empty_proofs() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
        let db = super::RocksDB::new(tmp_dir.to_owned());

        let new_proofs = db.get_used_proofs()?;
        assert!(new_proofs.is_empty());
        Ok(())
    }

    #[test]
    fn test_read_write_pending_invoices() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
        let db = super::RocksDB::new(tmp_dir.to_owned());

        let key = "foo";
        let invoice = Invoice {
            amount: 21,
            payment_request: "bar".to_string(),
        };
        db.add_pending_invoice(key.to_string(), invoice.clone())?;
        let lookup_invoice = db.get_pending_invoice(key.to_string())?;

        assert_eq!(invoice, lookup_invoice);
        Ok(())
    }
}
