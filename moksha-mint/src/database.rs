use async_trait::async_trait;
use moksha_core::{
    primitives::{Bolt11MeltQuote, Bolt11MintQuote},
    proof::Proofs,
};

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::{error::MokshaMintError, model::Invoice};
#[cfg(test)]
use mockall::automock;

pub struct PostgresDB {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl PostgresDB {
    pub async fn new() -> Result<PostgresDB, sqlx::Error> {
        Ok(PostgresDB {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect(
                    &dotenvy::var("MINT_DB_URL")
                        .expect("environment variable MINT_DB_URL is not set"),
                )
                .await?,
        })
    }

    pub async fn migrate(&self) {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .expect("Could not run migrations");
    }

    pub async fn start_transaction(
        &self,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn commit_transaction(
        &self,
        transaction: sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        transaction.commit().await
    }
}

#[async_trait]
impl Database for PostgresDB {
    async fn get_used_proofs(&self) -> Result<Proofs, MokshaMintError> {
        // let proofs = sqlx::query!("SELECT * FROM used_proofs")
        //     .fetch_all(&self.pool)
        //     .await?
        //     .into_iter()
        //     .map(|mut row| {
        //         proof.c = dhke::public_key_from_hex(&row.get).to_owned();
        //         Ok(proof)
        //     })
        //     .collect::<Result<Vec<Proof>, _>>()?;
        // FIXME
        Ok(Proofs::empty())
    }

    async fn add_used_proofs(&self, proofs: &Proofs) -> Result<(), MokshaMintError> {
        for proof in proofs.proofs() {
            sqlx::query!(
                "INSERT INTO used_proofs (amount, secret, c, keyset_id) VALUES ($1, $2, $3, $4)",
                proof.amount as i64,
                proof.secret,
                proof.c.to_string(),
                proof.keyset_id.to_string()
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn get_pending_invoice(
        &self,
        payment_request: String,
    ) -> Result<Invoice, MokshaMintError> {
        let invoice: Invoice = sqlx::query!(
            "SELECT amount, payment_request FROM pending_invoices WHERE payment_request = $1",
            payment_request
        )
        .map(|row| Invoice {
            amount: row.amount as u64,
            payment_request: row.payment_request,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    async fn add_pending_invoice(&self, invoice: &Invoice) -> Result<(), MokshaMintError> {
        Ok(())
    }

    async fn delete_pending_invoice(&self, payment_request: String) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM pending_invoices WHERE payment_request = $1",
            payment_request
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_bolt11_mint_quote(&self, id: &Uuid) -> Result<Bolt11MintQuote, MokshaMintError> {
        let quote: Bolt11MintQuote = sqlx::query!(
            "SELECT id, payment_request, expiry, paid FROM bolt11_mint_quotes WHERE id = $1",
            id
        )
        .map(|row| Bolt11MintQuote {
            quote_id: row.id,
            payment_request: row.payment_request,
            expiry: row.expiry as u64,
            paid: row.paid,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(quote)
    }

    async fn add_bolt11_mint_quote(&self, quote: &Bolt11MintQuote) -> Result<(), MokshaMintError> {
        // Implement the method here
        Ok(())
    }

    async fn update_bolt11_mint_quote(
        &self,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        // Implement the method here
        Ok(())
    }

    async fn delete_bolt11_mint_quote(
        &self,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM bolt11_mint_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_bolt11_melt_quote(&self, key: &Uuid) -> Result<Bolt11MeltQuote, MokshaMintError> {
        let quote: Bolt11MeltQuote = sqlx::query!(
            "SELECT id, payment_request, expiry, paid, amount, fee_reserve FROM bolt11_melt_quotes WHERE id = $1",
            key
        )
        .map(|row| Bolt11MeltQuote {
            quote_id: row.id,
            payment_request: row.payment_request,
            expiry: row.expiry as u64,
            paid: row.paid,
            amount: row.amount as u64,
            fee_reserve: row.fee_reserve as u64,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(quote)
    }

    async fn add_bolt11_melt_quote(&self, quote: &Bolt11MeltQuote) -> Result<(), MokshaMintError> {
        Ok(())
    }

    async fn update_bolt11_melt_quote(
        &self,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        Ok(())
    }

    async fn delete_bolt11_melt_quote(
        &self,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM bolt11_melt_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Database {
    async fn get_used_proofs(&self) -> Result<Proofs, MokshaMintError>;
    async fn add_used_proofs(&self, proofs: &Proofs) -> Result<(), MokshaMintError>;

    async fn get_pending_invoice(
        &self,
        payment_request: String,
    ) -> Result<Invoice, MokshaMintError>;
    async fn add_pending_invoice(&self, invoice: &Invoice) -> Result<(), MokshaMintError>;
    async fn delete_pending_invoice(&self, payment_request: String) -> Result<(), MokshaMintError>;

    async fn get_bolt11_mint_quote(&self, key: &Uuid) -> Result<Bolt11MintQuote, MokshaMintError>;
    async fn add_bolt11_mint_quote(&self, quote: &Bolt11MintQuote) -> Result<(), MokshaMintError>;
    async fn update_bolt11_mint_quote(
        &self,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError>;
    async fn delete_bolt11_mint_quote(
        &self,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_bolt11_melt_quote(&self, key: &Uuid) -> Result<Bolt11MeltQuote, MokshaMintError>;
    async fn add_bolt11_melt_quote(&self, quote: &Bolt11MeltQuote) -> Result<(), MokshaMintError>;
    async fn update_bolt11_melt_quote(
        &self,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError>;
    async fn delete_bolt11_melt_quote(
        &self,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError>;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use moksha_core::{
        dhke,
        proof::{Proof, Proofs},
    };
    use uuid::Uuid;

    use crate::{database::Database, model::Invoice};

    // #[test]
    // fn test_write_proofs() -> anyhow::Result<()> {
    //     let tmp = tempfile::tempdir()?;
    //     let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");

    //     let db: Arc<dyn Database> = Arc::new(super::RocksDB::new(tmp_dir.to_owned()));

    //     let keyset_id = "keyset_id";
    //     let proofs = Proofs::with_proof(Proof::new(
    //         21,
    //         "secret".to_string(),
    //         dhke::public_key_from_hex(
    //             "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
    //         ),
    //         keyset_id.to_owned(),
    //     ));

    //     db.add_used_proofs(&proofs)?;
    //     let new_proofs = db.get_used_proofs()?;
    //     assert_eq!(proofs, new_proofs);

    //     let proofs2 = Proofs::with_proof(Proof::new(
    //         42,
    //         "secret 2".to_string(),
    //         dhke::public_key_from_hex(
    //             "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
    //         ),
    //         keyset_id.to_owned(),
    //     ));

    //     db.add_used_proofs(&proofs2)?;
    //     let result_proofs = db.get_used_proofs()?;
    //     assert!(result_proofs.len() == 2);

    //     Ok(())
    // }

    // #[test]
    // fn test_read_empty_proofs() -> anyhow::Result<()> {
    //     let tmp = tempfile::tempdir()?;
    //     let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
    //     let db = super::RocksDB::new(tmp_dir.to_owned());

    //     let new_proofs = db.get_used_proofs()?;
    //     assert!(new_proofs.is_empty());
    //     Ok(())
    // }

    // #[test]
    // fn test_read_write_pending_invoices() -> anyhow::Result<()> {
    //     let tmp = tempfile::tempdir()?;
    //     let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
    //     let db = super::RocksDB::new(tmp_dir.to_owned());

    //     let key = "foo";
    //     let invoice = Invoice {
    //         amount: 21,
    //         payment_request: "bar".to_string(),
    //     };
    //     db.add_pending_invoice(key.to_string(), invoice.clone())?;
    //     let lookup_invoice = db.get_pending_invoice(key.to_string())?;

    //     assert_eq!(invoice, lookup_invoice);
    //     Ok(())
    // }

    // #[test]
    // fn test_read_write_quotes() -> anyhow::Result<()> {
    //     let tmp = tempfile::tempdir()?;
    //     let tmp_dir = tmp.path().to_str().expect("Could not create tmp dir");
    //     let db = super::RocksDB::new(tmp_dir.to_owned());

    //     let key = Uuid::new_v4();
    //     let quote = Quote::Bolt11Mint {
    //         quote_id: key,
    //         payment_request: "12345678".to_owned(),
    //         expiry: 12345678,
    //         paid: false,
    //     };

    //     db.add_quote(key.to_string(), quote.clone())?;
    //     let lookup_quote = db.get_quote(key.to_string())?;

    //     assert_eq!(quote, lookup_quote);
    //     Ok(())
    // }
}
