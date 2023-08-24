use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::model::{
    BlindedMessage, CheckFeesResponse, Keysets, PaymentRequest, PostMeltResponse, PostMintResponse,
    PostSplitResponse, Proofs,
};
use secp256k1::PublicKey;
use url::Url;

use crate::error::MokshaWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod reqwest;

#[async_trait(?Send)]
pub trait Client {
    async fn post_split_tokens(
        &self,
        mint_url: &Url,
        amount: u64,
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
}
