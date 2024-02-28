use async_trait::async_trait;
use moksha_core::{
    primitives::{Bolt11MeltQuote, Bolt11MintQuote, OnchainMeltQuote, OnchainMintQuote},
    proof::Proofs,
};
use uuid::Uuid;

use crate::{error::MokshaMintError, model::Invoice};

pub mod postgres;

#[async_trait]
pub trait Database {
    type DB: sqlx::Database;
    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, sqlx::Error>;
    async fn get_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MokshaMintError>;
    async fn add_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaMintError>;

    async fn get_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<Invoice, MokshaMintError>;
    async fn add_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
        invoice: &Invoice,
    ) -> Result<(), MokshaMintError>;
    async fn delete_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<(), MokshaMintError>;

    async fn get_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<Bolt11MintQuote, MokshaMintError>;
    async fn add_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError>;
    async fn update_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError>;
    async fn delete_bolt11_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<Bolt11MeltQuote, MokshaMintError>;
    async fn add_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError>;
    async fn update_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn delete_bolt11_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &Bolt11MeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMintQuote, MokshaMintError>;

    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn delete_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMeltQuote, MokshaMintError>;

    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn update_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn delete_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;
}
