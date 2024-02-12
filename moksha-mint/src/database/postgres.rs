use async_trait::async_trait;
use moksha_core::{
    dhke,
    primitives::{
        Bolt11MeltQuote, Bolt11MintQuote, CurrencyUnit, OnchainMeltQuote, OnchainMintQuote,
    },
    proof::{Proof, Proofs},
};

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::{config::DatabaseConfig, error::MokshaMintError, model::Invoice};

use super::Database;

pub struct PostgresDB {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl PostgresDB {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5) // FIXME make max connections configurable
                .connect(config.db_url.as_str())
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
        let proofs = sqlx::query!("SELECT * FROM used_proofs")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| Proof {
                amount: row.amount as u64,
                secret: row.secret,
                c: dhke::public_key_from_hex(&row.c).to_owned(),
                keyset_id: row.keyset_id,
                script: None,
            })
            .collect::<Vec<Proof>>();

        Ok(proofs.into())
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

    async fn get_pending_invoice(&self, key: String) -> Result<Invoice, MokshaMintError> {
        let invoice: Invoice = sqlx::query!(
            "SELECT amount, payment_request FROM pending_invoices WHERE key = $1",
            key
        )
        .map(|row| Invoice {
            amount: row.amount as u64,
            payment_request: row.payment_request,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    async fn add_pending_invoice(
        &self,
        key: String,
        invoice: &Invoice,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO pending_invoices (key, amount, payment_request) VALUES ($1, $2, $3)",
            key,
            invoice.amount as i64,
            invoice.payment_request
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_pending_invoice(&self, key: String) -> Result<(), MokshaMintError> {
        sqlx::query!("DELETE FROM pending_invoices WHERE key = $1", key)
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
        sqlx::query!(
            "INSERT INTO bolt11_mint_quotes (id, payment_request, expiry, paid) VALUES ($1, $2, $3, $4)",
            quote.quote_id,
            quote.payment_request,
            quote.expiry as i64,
            quote.paid
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_bolt11_mint_quote(
        &self,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE bolt11_mint_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
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
        sqlx::query!(
            "INSERT INTO bolt11_melt_quotes (id, payment_request, expiry, paid, amount, fee_reserve) VALUES ($1, $2, $3, $4, $5, $6)",
            quote.quote_id,
            quote.payment_request,
            quote.expiry as i64,
            quote.paid,
            quote.amount as i64,
            quote.fee_reserve as i64
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_bolt11_melt_quote(
        &self,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE bolt11_melt_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
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

    async fn get_onchain_mint_quote(
        &self,
        key: &Uuid,
    ) -> Result<OnchainMintQuote, MokshaMintError> {
        let quote: OnchainMintQuote = sqlx::query!(
            "SELECT id, address, amount, expiry, paid  FROM onchain_mint_quotes WHERE id = $1",
            key
        )
        .map(|row| OnchainMintQuote {
            quote_id: row.id,
            address: row.address,
            expiry: row.expiry as u64,
            paid: row.paid,
            amount: row.amount as u64,
            unit: CurrencyUnit::Sat,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(quote)
    }
    async fn add_onchain_mint_quote(
        &self,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO onchain_mint_quotes (id, address, amount, expiry, paid) VALUES ($1, $2, $3, $4, $5)",
            quote.quote_id,
            quote.address,
            quote.amount as i64,
            quote.expiry as i64,
            quote.paid,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_onchain_mint_quote(
        &self,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE onchain_mint_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_onchain_mint_quote(
        &self,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM onchain_mint_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_onchain_melt_quote(
        &self,
        key: &Uuid,
    ) -> Result<OnchainMeltQuote, MokshaMintError> {
        let quote: OnchainMeltQuote = sqlx::query!(
            "SELECT id, amount,address, fee_total, fee_sat_per_vbyte, expiry, paid  FROM onchain_melt_quotes WHERE id = $1",
            key
        )
        .map(|row| OnchainMeltQuote {
            quote_id: row.id,
            address: row.address,
            amount: row.amount as u64,
            fee_total: row.fee_total as u64,
            fee_sat_per_vbyte: row.fee_sat_per_vbyte as u32,
            expiry: row.expiry as u64,
            paid: row.paid,
        })
        .fetch_one(&self.pool)
        .await?;

        Ok(quote)
    }
    async fn add_onchain_melt_quote(
        &self,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO onchain_melt_quotes (id, amount, address, fee_total, fee_sat_per_vbyte, expiry, paid) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            quote.quote_id,
            quote.amount as i64,
            quote.address,
            quote.fee_total as i64,
            quote.fee_sat_per_vbyte as i64,
            quote.expiry as i64,
            quote.paid,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn update_onchain_melt_quote(
        &self,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE onchain_melt_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn delete_onchain_melt_quote(
        &self,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM onchain_melt_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
