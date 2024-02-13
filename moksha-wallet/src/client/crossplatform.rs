use async_trait::async_trait;

use moksha_core::{
    blind::BlindedMessage,
    keyset::V1Keysets,
    primitives::{
        CurrencyUnit, GetMeltOnchainResponse, KeysResponse, MintInfoResponse,
        PostMeltBolt11Request, PostMeltBolt11Response, PostMeltOnchainRequest,
        PostMeltOnchainResponse, PostMeltQuoteBolt11Request, PostMeltQuoteBolt11Response,
        PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse, PostMintBolt11Request,
        PostMintBolt11Response, PostMintOnchainRequest, PostMintOnchainResponse,
        PostMintQuoteBolt11Request, PostMintQuoteBolt11Response, PostMintQuoteOnchainRequest,
        PostMintQuoteOnchainResponse, PostSwapRequest, PostSwapResponse,
    },
    proof::Proofs,
};

use url::Url;

use crate::{error::MokshaWalletError, http::CrossPlatformHttpClient};

use super::CashuClient;

#[async_trait(?Send)]
impl CashuClient for CrossPlatformHttpClient {
    async fn get_keys(&self, mint_url: &Url) -> Result<KeysResponse, MokshaWalletError> {
        self.do_get(&mint_url.join("v1/keys")?).await
    }

    async fn get_keys_by_id(
        &self,
        mint_url: &Url,
        keyset_id: String,
    ) -> Result<KeysResponse, MokshaWalletError> {
        self.do_get(&mint_url.join(&format!("v1/keys/{}", keyset_id))?)
            .await
    }

    async fn get_keysets(&self, mint_url: &Url) -> Result<V1Keysets, MokshaWalletError> {
        self.do_get(&mint_url.join("v1/keysets")?).await
    }

    async fn post_swap(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse, MokshaWalletError> {
        let body = PostSwapRequest { inputs, outputs };

        self.do_post(&mint_url.join("v1/swap")?, &body).await
    }

    async fn post_melt_bolt11(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        quote: String,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltBolt11Response, MokshaWalletError> {
        let body = PostMeltBolt11Request {
            quote,
            inputs,
            outputs,
        };

        self.do_post(&mint_url.join("v1/melt/bolt11")?, &body).await
    }

    async fn post_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        payment_request: String,
        unit: CurrencyUnit,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError> {
        let body = PostMeltQuoteBolt11Request {
            request: payment_request,
            unit,
        };

        self.do_post(&mint_url.join("v1/melt/quote/bolt11")?, &body)
            .await
    }

    async fn get_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError> {
        let url = mint_url.join(&format!("v1/melt/quote/bolt11/{}", quote))?;
        self.do_get(&url).await
    }

    async fn post_mint_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintBolt11Response, MokshaWalletError> {
        let body = PostMintBolt11Request {
            quote,
            outputs: blinded_messages,
        };
        self.do_post(&mint_url.join("v1/mint/bolt11")?, &body).await
    }

    async fn post_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        let body = PostMintQuoteBolt11Request { amount, unit };
        self.do_post(&mint_url.join("v1/mint/quote/bolt11")?, &body)
            .await
    }

    async fn get_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        self.do_get(&mint_url.join(&format!("v1/mint/quote/bolt11/{}", quote))?)
            .await
    }

    async fn post_mint_onchain(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintOnchainResponse, MokshaWalletError> {
        let body = PostMintOnchainRequest {
            quote,
            outputs: blinded_messages,
        };
        self.do_post(&mint_url.join("v1/mint/btconchain")?, &body)
            .await
    }

    async fn post_mint_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError> {
        let body = PostMintQuoteOnchainRequest { amount, unit };
        self.do_post(&mint_url.join("v1/mint/quote/btconchain")?, &body)
            .await
    }

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError> {
        self.do_get(&mint_url.join(&format!("v1/mint/quote/btconchain/{}", quote))?)
            .await
    }

    async fn get_info(&self, mint_url: &Url) -> Result<MintInfoResponse, MokshaWalletError> {
        self.do_get(&mint_url.join("v1/info")?).await
    }

    async fn is_v1_supported(&self, mint_url: &Url) -> Result<bool, MokshaWalletError> {
        self.get_status(&mint_url.join("v1/info")?)
            .await
            .map(|s| s == 200)
    }

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        quote: String,
    ) -> Result<PostMeltOnchainResponse, MokshaWalletError> {
        let body = PostMeltOnchainRequest { quote, inputs };
        self.do_post(&mint_url.join("v1/melt/btconchain")?, &body)
            .await
    }

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<Vec<PostMeltQuoteOnchainResponse>, MokshaWalletError> {
        let body = PostMeltQuoteOnchainRequest {
            address,
            amount,
            unit,
        };
        self.do_post(&mint_url.join("v1/melt/quote/btconchain")?, &body)
            .await
    }

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteOnchainResponse, MokshaWalletError> {
        self.do_get(&mint_url.join(&format!("/v1/melt/quote/btconchain/{quote}"))?)
            .await
    }

    async fn get_melt_onchain(
        &self,
        mint_url: &Url,
        txid: String,
    ) -> Result<GetMeltOnchainResponse, MokshaWalletError> {
        self.do_get(&mint_url.join(&format!("/v1/melt/btconchain/{txid}"))?)
            .await
    }
}
