#![allow(clippy::blocks_in_conditions)]
use async_trait::async_trait;
use moksha_core::{
    dhke,
    primitives::{
        Bolt11MeltQuote, Bolt11MintQuote, BtcOnchainMeltQuote, BtcOnchainMintQuote, CurrencyUnit,
    },
    proof::{Proof, Proofs},
};

use crate::{config::DatabaseConfig, error::MokshaMintError, model::Invoice};
use moksha_core::primitives::{BitcreditMintQuote, BitcreditRequestToMint};
use sqlx::postgres::PgPoolOptions;
use tracing::instrument;
use uuid::Uuid;

use super::Database;

#[derive(Clone)]
pub struct PostgresDB {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl PostgresDB {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(config.max_connections)
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
}

#[async_trait]
impl Database for PostgresDB {
    type DB = sqlx::Postgres;

    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, sqlx::Error> {
        self.pool.begin().await
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MokshaMintError> {
        let proofs = sqlx::query!("SELECT * FROM used_proofs")
            .fetch_all(&mut **tx)
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

    #[instrument(level = "debug", skip(self), err)]
    async fn add_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaMintError> {
        for proof in proofs.proofs() {
            sqlx::query!(
                "INSERT INTO used_proofs (amount, secret, c, keyset_id) VALUES ($1, $2, $3, $4)",
                proof.amount as i64,
                proof.secret,
                proof.c.to_string(),
                proof.keyset_id.to_string()
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<Invoice, MokshaMintError> {
        let invoice: Invoice = sqlx::query!(
            "SELECT amount, payment_request FROM pending_invoices WHERE key = $1",
            key
        )
        .map(|row| Invoice {
            amount: row.amount as u64,
            payment_request: row.payment_request,
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(invoice)
    }

    #[instrument(level = "debug", skip(self))]
    async fn add_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
        invoice: &Invoice,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO pending_invoices (key, amount, payment_request) VALUES ($1, $2, $3)",
            key,
            invoice.amount as i64,
            invoice.payment_request
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!("DELETE FROM pending_invoices WHERE key = $1", key)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        id: &Uuid,
    ) -> Result<Bolt11MintQuote, MokshaMintError> {
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
        .fetch_one(&mut **tx)
        .await?;
        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        id: &Uuid,
    ) -> Result<BitcreditMintQuote, MokshaMintError> {
        let quote: BitcreditMintQuote = sqlx::query!(
            "SELECT id, bill_id, node_id, sent FROM bitcredit_mint_quotes WHERE id = $1",
            id
        )
        .map(|row| BitcreditMintQuote {
            quote_id: row.id,
            bill_id: row.bill_id,
            node_id: row.node_id,
            sent: row.sent,
        })
        .fetch_one(&mut **tx)
        .await?;
        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO bolt11_mint_quotes (id, payment_request, expiry, paid) VALUES ($1, $2, $3, $4)",
            quote.quote_id,
            quote.payment_request,
            quote.expiry as i64,
            quote.paid
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE bolt11_mint_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BitcreditMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE bitcredit_mint_quotes SET sent = $1 WHERE id = $2",
            quote.sent,
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM bolt11_mint_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BitcreditMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO bitcredit_mint_quotes (id, bill_id, node_id, sent) VALUES ($1, $2, $3, $4)",
            quote.quote_id,
            quote.bill_id,
            quote.node_id,
            quote.sent,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_bitcredit_request_to_mint(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BitcreditRequestToMint,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO bitcredit_requests_to_mint (bill_id, bill_key) VALUES ($1, $2)",
            quote.bill_id,
            quote.bill_key,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<Bolt11MeltQuote, MokshaMintError> {
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
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO bolt11_melt_quotes (id, payment_request, expiry, paid, amount, fee_reserve) VALUES ($1, $2, $3, $4, $5, $6)",
            quote.quote_id,
            quote.payment_request,
            quote.expiry as i64,
            quote.paid,
            quote.amount as i64,
            quote.fee_reserve as i64
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE bolt11_melt_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM bolt11_melt_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<BtcOnchainMintQuote, MokshaMintError> {
        let quote: BtcOnchainMintQuote = sqlx::query!(
            "SELECT id, address, amount, expiry, paid  FROM onchain_mint_quotes WHERE id = $1",
            key
        )
        .map(|row| BtcOnchainMintQuote {
            quote_id: row.id,
            address: row.address,
            expiry: row.expiry as u64,
            paid: row.paid,
            amount: row.amount as u64,
            unit: CurrencyUnit::Sat,
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO onchain_mint_quotes (id, address, amount, expiry, paid) VALUES ($1, $2, $3, $4, $5)",
            quote.quote_id,
            quote.address,
            quote.amount as i64,
            quote.expiry as i64,
            quote.paid,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE onchain_mint_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM onchain_mint_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<BtcOnchainMeltQuote, MokshaMintError> {
        let quote: BtcOnchainMeltQuote = sqlx::query!(
            "SELECT id, amount,address, fee_total, fee_sat_per_vbyte, expiry, paid, description  FROM onchain_melt_quotes WHERE id = $1",
            key
        )
        .map(|row| BtcOnchainMeltQuote {
            quote_id: row.id,
            address: row.address,
            amount: row.amount as u64,
            fee_total: row.fee_total as u64,
            fee_sat_per_vbyte: row.fee_sat_per_vbyte as u32,
            expiry: row.expiry as u64,
            paid: row.paid,
            description: row.description
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "INSERT INTO onchain_melt_quotes (id, amount, address, fee_total, fee_sat_per_vbyte, expiry, paid, description) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            quote.quote_id,
            quote.amount as i64,
            quote.address,
            quote.fee_total as i64,
            quote.fee_sat_per_vbyte as i64,
            quote.expiry as i64,
            quote.paid,
            quote.description
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "UPDATE onchain_melt_quotes SET paid = $1 WHERE id = $2",
            quote.paid,
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError> {
        sqlx::query!(
            "DELETE FROM onchain_melt_quotes WHERE id = $1",
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
