use async_trait::async_trait;
use moksha_core::{
    primitives::{Bolt11MeltQuote, Bolt11MintQuote, OnchainMeltQuote, OnchainMintQuote},
    proof::Proofs,
};
use uuid::Uuid;

use crate::{error::MokshaMintError, model::Invoice};

pub mod postgres;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Database {
    async fn get_used_proofs(&self) -> Result<Proofs, MokshaMintError>;
    async fn add_used_proofs(&self, proofs: &Proofs) -> Result<(), MokshaMintError>;

    async fn get_pending_invoice(&self, key: String) -> Result<Invoice, MokshaMintError>;
    async fn add_pending_invoice(
        &self,
        key: String,
        invoice: &Invoice,
    ) -> Result<(), MokshaMintError>;
    async fn delete_pending_invoice(&self, key: String) -> Result<(), MokshaMintError>;

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

    async fn get_onchain_mint_quote(&self, key: &Uuid)
        -> Result<OnchainMintQuote, MokshaMintError>;
    async fn add_onchain_mint_quote(&self, quote: &OnchainMintQuote)
        -> Result<(), MokshaMintError>;
    async fn update_onchain_mint_quote(
        &self,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError>;
    async fn delete_onchain_mint_quote(
        &self,
        quote: &OnchainMintQuote,
    ) -> Result<(), MokshaMintError>;

    async fn get_onchain_melt_quote(&self, key: &Uuid)
        -> Result<OnchainMeltQuote, MokshaMintError>;
    async fn add_onchain_melt_quote(&self, quote: &OnchainMeltQuote)
        -> Result<(), MokshaMintError>;
    async fn update_onchain_melt_quote(
        &self,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;
    async fn delete_onchain_melt_quote(
        &self,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MokshaMintError>;
}
