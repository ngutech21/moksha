use std::collections::HashMap;

use async_trait::async_trait;
use gloo_net::http::{Request, Response};
use moksha_core::model::CashuErrorResponse;
use moksha_core::model::{
    BlindedMessage, CheckFeesRequest, CheckFeesResponse, Keysets, PaymentRequest, PostMeltRequest,
    PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
};
use moksha_core::proof::Proofs;
use moksha_wallet::{client::Client, error::MokshaWalletError};
use secp256k1::PublicKey;
use url::Url;

#[derive(Debug, Clone)]
pub struct WasmClient;

impl WasmClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl Client for WasmClient {
    async fn post_split_tokens(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError> {
        let body = &PostSplitRequest { proofs, outputs };

        let resp = Request::post(mint_url.join("split")?.as_str())
            .header("content-type", "application/json")
            .json(body)?
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
        let body = &PostMeltRequest {
            pr,
            proofs,
            outputs,
        };

        let resp = Request::post(mint_url.join("melt")?.as_str())
            .header("content-type", "application/json")
            .json(body)?
            .send()
            .await?;
        extract_response_data::<PostMeltResponse>(resp).await
    }

    async fn post_checkfees(
        &self,
        mint_url: &Url,
        pr: String,
    ) -> Result<CheckFeesResponse, MokshaWalletError> {
        let resp = Request::post(mint_url.join("checkfees")?.as_str())
            .header("content-type", "application/json")
            .json(&CheckFeesRequest { pr })?
            .send()
            .await?;

        extract_response_data::<CheckFeesResponse>(resp).await
    }

    async fn get_mint_keys(
        &self,
        mint_url: &Url,
    ) -> Result<HashMap<u64, PublicKey>, MokshaWalletError> {
        let resp = Request::get(mint_url.join("keys")?.as_str()).send().await?;
        extract_response_data::<HashMap<u64, PublicKey>>(resp).await
    }

    async fn get_mint_keysets(&self, mint_url: &Url) -> Result<Keysets, MokshaWalletError> {
        let resp = Request::get(mint_url.join("keysets")?.as_str())
            .send()
            .await?;
        extract_response_data::<Keysets>(resp).await
    }

    async fn get_mint_payment_request(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError> {
        let resp = Request::get(mint_url.join(&format!("mint?amount={}", amount))?.as_str())
            .send()
            .await?;
        extract_response_data::<PaymentRequest>(resp).await
    }

    async fn post_mint_payment_request(
        &self,
        mint_url: &Url,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, MokshaWalletError> {
        let body = &PostMintRequest {
            outputs: blinded_messages,
        };

        let resp = Request::post(mint_url.join(&format!("mint?hash={}", hash))?.as_str())
            .header("content-type", "application/json")
            .json(body)?
            .send()
            .await?;
        extract_response_data::<PostMintResponse>(resp).await
    }
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
            let txt = response.text().await.unwrap(); // FIXME handle error
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
