use async_trait::async_trait;
use moksha_core::{
    blind::BlindedMessage,
    keyset::V1Keysets,
    primitives::{
        CurrencyUnit, GetMeltOnchainResponse, KeysResponse, MintInfoResponse,
        PostMeltBolt11Response, PostMeltOnchainResponse, PostMeltQuoteBolt11Response,
        PostMeltQuoteOnchainResponse, PostMintBolt11Response, PostMintOnchainResponse,
        PostMintQuoteBolt11Response, PostMintQuoteOnchainResponse, PostSwapResponse,
    },
    proof::Proofs,
};

use url::Url;

use crate::error::MokshaWalletError;

pub mod crossplatform;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait(?Send)]
pub trait CashuClient {
    async fn get_keys(&self, mint_url: &Url) -> Result<KeysResponse, MokshaWalletError>;

    async fn get_keys_by_id(
        &self,
        mint_url: &Url,
        keyset_id: String,
    ) -> Result<KeysResponse, MokshaWalletError>;

    async fn get_keysets(&self, mint_url: &Url) -> Result<V1Keysets, MokshaWalletError>;

    async fn post_swap(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        output: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse, MokshaWalletError>;

    async fn post_melt_bolt11(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        quote: String,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltBolt11Response, MokshaWalletError>;

    async fn post_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        payment_request: String,
        unit: CurrencyUnit,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError>;

    async fn get_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError>;

    async fn post_mint_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintBolt11Response, MokshaWalletError>;

    async fn post_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError>;

    async fn get_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError>;

    async fn get_info(&self, mint_url: &Url) -> Result<MintInfoResponse, MokshaWalletError>;

    async fn is_v1_supported(&self, mint_url: &Url) -> Result<bool, MokshaWalletError>;

    async fn post_mint_onchain(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintOnchainResponse, MokshaWalletError>;

    async fn post_mint_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError>;

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError>;

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        quote: String,
    ) -> Result<PostMeltOnchainResponse, MokshaWalletError>;

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<Vec<PostMeltQuoteOnchainResponse>, MokshaWalletError>;

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteOnchainResponse, MokshaWalletError>;

    async fn get_melt_onchain(
        &self,
        mint_url: &Url,
        txid: String,
    ) -> Result<GetMeltOnchainResponse, MokshaWalletError>;
}
