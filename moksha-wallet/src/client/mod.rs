use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::{
    blind::BlindedMessage,
    keyset::{Keysets, V1Keysets},
    primitives::{
        CheckFeesResponse, CurrencyUnit, KeysResponse, MintInfoResponse, MintLegacyInfoResponse,
        PaymentRequest, PostMeltBolt11Response, PostMeltQuoteBolt11Response, PostMeltResponse,
        PostMintBolt11Response, PostMintQuoteBolt11Response, PostMintResponse, PostSplitResponse,
        PostSwapResponse,
    },
    proof::Proofs,
};
use secp256k1::PublicKey;
use url::Url;

use crate::error::MokshaWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod reqwest;

#[async_trait(?Send)]
pub trait LegacyClient {
    async fn post_split_tokens(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        output: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError>;

    async fn post_mint_payment_request(
        &self,
        mint_url: &Url,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, MokshaWalletError>;

    async fn post_melt_tokens(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        pr: String,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltResponse, MokshaWalletError>;

    async fn post_checkfees(
        &self,
        mint_url: &Url,
        pr: String,
    ) -> Result<CheckFeesResponse, MokshaWalletError>;

    async fn get_mint_keys(
        &self,
        mint_url: &Url,
    ) -> Result<HashMap<u64, PublicKey>, MokshaWalletError>;

    async fn get_mint_keysets(&self, mint_url: &Url) -> Result<Keysets, MokshaWalletError>;

    async fn get_mint_payment_request(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError>;

    async fn get_info(&self, mint_url: &Url) -> Result<MintLegacyInfoResponse, MokshaWalletError>;
}

#[async_trait(?Send)]
pub trait Client {
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
}
