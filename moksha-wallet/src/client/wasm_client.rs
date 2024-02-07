use async_trait::async_trait;
use gloo_net::http::{Request, Response};

use moksha_core::{
    blind::BlindedMessage,
    keyset::V1Keysets,
    primitives::{
        CashuErrorResponse, CurrencyUnit, GetMeltOnchainResponse, KeysResponse, MintInfoResponse,
        PostMeltBolt11Request, PostMeltBolt11Response, PostMeltOnchainRequest,
        PostMeltOnchainResponse, PostMeltQuoteBolt11Request, PostMeltQuoteBolt11Response,
        PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse, PostMintBolt11Request,
        PostMintBolt11Response, PostMintOnchainRequest, PostMintOnchainResponse,
        PostMintQuoteBolt11Request, PostMintQuoteBolt11Response, PostMintQuoteOnchainRequest,
        PostMintQuoteOnchainResponse, PostSwapRequest, PostSwapResponse,
    },
    proof::Proofs,
};

use crate::error::MokshaWalletError;
use url::Url;

use super::Client;

#[derive(Debug, Clone)]
pub struct WasmClient;

impl WasmClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl Client for WasmClient {
    async fn get_keys(&self, mint_url: &Url) -> Result<KeysResponse, MokshaWalletError> {
        do_get(&mint_url.join("v1/keys")?).await
    }

    async fn get_keys_by_id(
        &self,
        mint_url: &Url,
        keyset_id: String,
    ) -> Result<KeysResponse, MokshaWalletError> {
        do_get(&mint_url.join(&format!("v1/keys/{}", keyset_id))?).await
    }

    async fn get_keysets(&self, mint_url: &Url) -> Result<V1Keysets, MokshaWalletError> {
        do_get(&mint_url.join("v1/keysets")?).await
    }

    async fn post_swap(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse, MokshaWalletError> {
        let body = PostSwapRequest { inputs, outputs };

        do_post(&mint_url.join("v1/swap")?, &body).await
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

        do_post(&mint_url.join("v1/melt/bolt11")?, &body).await
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

        do_post(&mint_url.join("v1/melt/quote/bolt11")?, &body).await
    }

    async fn get_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError> {
        let url = mint_url.join(&format!("v1/melt/quote/bolt11/{}", quote))?;
        do_get(&url).await
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
        do_post(&mint_url.join("v1/mint/bolt11")?, &body).await
    }

    async fn post_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        let body = PostMintQuoteBolt11Request { amount, unit };
        do_post(&mint_url.join("v1/mint/quote/bolt11")?, &body).await
    }

    async fn get_mint_quote_bolt11(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        do_get(&mint_url.join(&format!("v1/mint/quote/bolt11/{}", quote))?).await
    }

    async fn get_info(&self, mint_url: &Url) -> Result<MintInfoResponse, MokshaWalletError> {
        do_get(&mint_url.join("v1/info")?).await
    }

    async fn is_v1_supported(&self, mint_url: &Url) -> Result<bool, MokshaWalletError> {
        let resp = Request::get(mint_url.join("v1/info")?.as_str())
            .send()
            .await?;

        Ok(resp.status() == 200)
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
        do_post(&mint_url.join("v1/mint/btconchain")?, &body).await
    }

    async fn post_mint_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError> {
        do_post(
            &mint_url.join("v1/mint/quote/btconchain")?,
            &PostMintQuoteOnchainRequest { amount, unit },
        )
        .await
    }

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError> {
        do_get(&mint_url.join(&format!("v1/mint/quote/btconchain/{}", quote))?).await
    }

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        quote: String,
    ) -> Result<PostMeltOnchainResponse, MokshaWalletError> {
        do_post(
            &mint_url.join("v1/melt/btconchain")?,
            &PostMeltOnchainRequest { quote, inputs },
        )
        .await
    }

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMeltQuoteOnchainResponse, MokshaWalletError> {
        let body = PostMeltQuoteOnchainRequest {
            address,
            amount,
            unit,
        };
        do_post(&mint_url.join("v1/melt/quote/btconchain")?, &body).await
    }

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteOnchainResponse, MokshaWalletError> {
        do_get(&mint_url.join(&format!("/v1/melt/quote/btconchain/{quote}"))?).await
    }

    async fn get_melt_onchain(
        &self,
        mint_url: &Url,
        txid: String,
    ) -> Result<GetMeltOnchainResponse, MokshaWalletError> {
        do_get(&mint_url.join(&format!("/v1/melt/btconchain/{txid}"))?).await
    }
}

async fn do_get<T: serde::de::DeserializeOwned>(url: &Url) -> Result<T, MokshaWalletError> {
    let resp = Request::get(url.as_str()).send().await?;
    extract_response_data::<T>(resp).await
}

async fn do_post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
    url: &Url,
    body: &B,
) -> Result<T, MokshaWalletError> {
    let resp = Request::post(url.as_str())
        .header("content-type", "application/json")
        .json(body)?
        .send()
        .await?;
    extract_response_data::<T>(resp).await
}

async fn extract_response_data<T: serde::de::DeserializeOwned>(
    response: Response,
) -> Result<T, MokshaWalletError> {
    match response.status() {
        200 => {
            let response_text = response.text().await.unwrap(); // FIXME handle error
            match serde_json::from_str::<T>(&response_text) {
                Ok(data) => Ok(data),
                Err(_) => {
                    let data = serde_json::from_str::<CashuErrorResponse>(&response_text)
                        .map_err(|_| MokshaWalletError::UnexpectedResponse(response_text))
                        .unwrap();

                    // FIXME: use the error code to return a proper error
                    match data.detail.as_str() {
                        "Lightning invoice not paid yet." => {
                            Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.detail))
                        }
                        _ => Err(MokshaWalletError::MintError(data.detail)),
                    }
                }
            }
        }
        _ => {
            let txt = response.text().await.unwrap(); // FIXME handle error
            let data = serde_json::from_str::<CashuErrorResponse>(&txt)
                .map_err(|_| MokshaWalletError::UnexpectedResponse(txt))
                .unwrap();

            // FIXME: use the error code to return a proper error
            match data.detail.as_str() {
                "Lightning invoice not paid yet." => {
                    Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.detail))
                }
                _ => Err(MokshaWalletError::MintError(data.detail)),
            }
        }
    }
}
