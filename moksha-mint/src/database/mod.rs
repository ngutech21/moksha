use crate::{error::MokshaMintError, model::Invoice};
use async_trait::async_trait;
use moksha_core::primitives::{BitcreditMintQuote, BitcreditQuoteCheck, BitcreditRequestToMint};
use moksha_core::{
    primitives::{Bolt11MeltQuote, Bolt11MintQuote, BtcOnchainMeltQuote, BtcOnchainMintQuote},
    proof::Proofs,
};
use uuid::Uuid;

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

    async fn get_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<BitcreditMintQuote, MokshaMintError>;
    async fn add_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BitcreditMintQuote,
    ) -> Result<(), MokshaMintError>;
    async fn update_bitcredit_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BitcreditMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn add_bitcredit_request_to_mint(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        request_to_mint: &BitcreditRequestToMint,
    ) -> Result<(), MokshaMintError>;

    async fn check_bitcredit_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote_check: &BitcreditQuoteCheck,
    ) -> Result<BitcreditMintQuote, MokshaMintError>;

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
    ) -> Result<BtcOnchainMintQuote, MokshaMintError>;

    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn delete_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<BtcOnchainMeltQuote, MokshaMintError>;

    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn update_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;

    async fn delete_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;
}
