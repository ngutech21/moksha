use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::{
    blind::BlindedMessage,
    keyset::{Keysets, V1Keysets},
    primitives::{
        CashuErrorResponse, CheckFeesRequest, CheckFeesResponse, CurrencyUnit,
        GetMeltOnchainResponse, KeysResponse, MintInfoResponse, MintLegacyInfoResponse,
        PaymentRequest, PostMeltBolt11Request, PostMeltBolt11Response, PostMeltOnchainRequest,
        PostMeltOnchainResponse, PostMeltQuoteBolt11Request, PostMeltQuoteBolt11Response,
        PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse, PostMeltRequest,
        PostMeltResponse, PostMintBolt11Request, PostMintBolt11Response, PostMintOnchainRequest,
        PostMintOnchainResponse, PostMintQuoteBolt11Request, PostMintQuoteBolt11Response,
        PostMintQuoteOnchainRequest, PostMintQuoteOnchainResponse, PostMintRequest,
        PostMintResponse, PostSplitRequest, PostSplitResponse, PostSwapRequest, PostSwapResponse,
    },
    proof::Proofs,
};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Response, StatusCode,
};
use secp256k1::PublicKey;

use crate::{client::LegacyClient, error::MokshaWalletError};
use url::Url;

use super::Client;

#[derive(Debug, Clone)]
pub struct HttpClient {
    request_client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            request_client: reqwest::Client::new(),
        }
    }

    async fn do_get<T: serde::de::DeserializeOwned>(
        &self,
        url: &Url,
    ) -> Result<T, MokshaWalletError> {
        let resp = self.request_client.get(url.clone()).send().await?;
        extract_response_data::<T>(resp).await
    }

    async fn do_post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &Url,
        body: &B,
    ) -> Result<T, MokshaWalletError> {
        let resp = self
            .request_client
            .post(url.clone())
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(serde_json::to_string(body)?)
            .send()
            .await?;
        extract_response_data::<T>(resp).await
    }
}
impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl LegacyClient for HttpClient {
    async fn post_split_tokens(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError> {
        let body = serde_json::to_string(&PostSplitRequest { proofs, outputs })?;

        let resp = self
            .request_client
            .post(mint_url.join("split")?)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;

        extract_response_data::<PostSplitResponse>(resp).await
    }

    async fn post_mint_payment_request(
        &self,
        mint_url: &Url,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, MokshaWalletError> {
        let url = mint_url.join(&format!("mint?hash={}", hash))?;
        let body = serde_json::to_string(&PostMintRequest {
            outputs: blinded_messages,
        })?;

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;
        extract_response_data::<PostMintResponse>(resp).await
    }

    async fn post_melt_tokens(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        pr: String,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltResponse, MokshaWalletError> {
        let body = serde_json::to_string(&PostMeltRequest {
            pr,
            proofs,
            outputs,
        })?;

        let resp = self
            .request_client
            .post(mint_url.join("melt")?)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;
        extract_response_data::<PostMeltResponse>(resp).await
    }

    async fn post_checkfees(
        &self,
        mint_url: &Url,
        pr: String,
    ) -> Result<CheckFeesResponse, MokshaWalletError> {
        let body = serde_json::to_string(&CheckFeesRequest { pr })?;

        let resp = self
            .request_client
            .post(mint_url.join("checkfees")?)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;

        extract_response_data::<CheckFeesResponse>(resp).await
    }

    async fn get_mint_keys(
        &self,
        mint_url: &Url,
    ) -> Result<HashMap<u64, PublicKey>, MokshaWalletError> {
        let resp = self
            .request_client
            .get(mint_url.join("keys")?)
            .send()
            .await?;
        extract_response_data::<HashMap<u64, PublicKey>>(resp).await
    }

    async fn get_mint_keysets(&self, mint_url: &Url) -> Result<Keysets, MokshaWalletError> {
        let resp = self
            .request_client
            .get(mint_url.join("keysets")?)
            .send()
            .await?;
        extract_response_data::<Keysets>(resp).await
    }

    async fn get_mint_payment_request(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError> {
        let url = mint_url.join(&format!("mint?amount={}", amount))?;
        let resp = self.request_client.get(url).send().await?;
        extract_response_data::<PaymentRequest>(resp).await
    }

    async fn get_info(&self, mint_url: &Url) -> Result<MintLegacyInfoResponse, MokshaWalletError> {
        let resp = self
            .request_client
            .get(mint_url.join("info")?)
            .send()
            .await?;
        extract_response_data::<MintLegacyInfoResponse>(resp).await
    }
}

#[async_trait(?Send)]
impl Client for HttpClient {
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
        let resp = self
            .request_client
            .get(mint_url.join("v1/info")?)
            .send()
            .await?;
        Ok(resp.status() == StatusCode::OK)
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
    ) -> Result<PostMeltQuoteOnchainResponse, MokshaWalletError> {
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

async fn extract_response_data<T: serde::de::DeserializeOwned>(
    response: Response,
) -> Result<T, MokshaWalletError> {
    match response.status() {
        StatusCode::OK => {
            let response_text = response.text().await?;
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
            let txt = response.text().await?;
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_deserialize_error() -> anyhow::Result<()> {
        let input = "{\"code\":0,\"detail\":\"Lightning invoice not paid yet.\"}";
        let data = serde_json::from_str::<super::CashuErrorResponse>(input)?;
        assert_eq!(data.code, 0);
        assert_eq!(data.detail, "Lightning invoice not paid yet.");
        Ok(())
    }
}
