use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::model::{
    BlindedMessage, CashuErrorResponse, CheckFeesRequest, CheckFeesResponse, InvoiceQuoteResult,
    Keysets, PaymentRequest, PostMeltRequest, PostMeltResponse, PostMintRequest, PostMintResponse,
    PostSplitRequest, PostSplitResponse, Proofs,
};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Response, StatusCode,
};
use secp256k1::PublicKey;

use crate::{client::Client, error::MokshaWalletError};
use url::Url;

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
}
impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Client for HttpClient {
    async fn post_split_tokens(
        &self,
        mint_url: &Url,
        amount: u64,
        proofs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError> {
        let body = serde_json::to_string(&PostSplitRequest {
            amount: Some(amount),
            proofs,
            outputs,
        })?;

        let resp = self
            .request_client
            .post(mint_url.join("split")?)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;

        extract_response_data::<PostSplitResponse>(resp).await
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

    async fn get_melt_tokens(
        &self,
        mint_url: &Url,
        pr: String,
    ) -> Result<InvoiceQuoteResult, MokshaWalletError> {
        let url = mint_url.join(&format!("melt/{}", pr))?;
        let resp = self.request_client.get(url).send().await?;
        extract_response_data::<InvoiceQuoteResult>(resp).await
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
                    match data.error.as_str() {
                        "Lightning invoice not paid yet." => {
                            Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.error))
                        }
                        _ => Err(MokshaWalletError::MintError(data.error)),
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
            match data.error.as_str() {
                "Lightning invoice not paid yet." => {
                    Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.error))
                }
                _ => Err(MokshaWalletError::MintError(data.error)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_deserialize_error() -> anyhow::Result<()> {
        let input = "{\"code\":0,\"error\":\"Lightning invoice not paid yet.\"}";
        let data = serde_json::from_str::<super::CashuErrorResponse>(input)?;
        assert_eq!(data.code, 0);
        assert_eq!(data.error, "Lightning invoice not paid yet.");
        Ok(())
    }
}
