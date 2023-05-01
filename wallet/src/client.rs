use std::collections::HashMap;

use async_trait::async_trait;
use cashurs_core::model::{
    BlindedMessage, CheckFeesRequest, CheckFeesResponse, Keysets, PaymentRequest, PostMeltRequest,
    PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
    Proofs,
};
#[cfg(test)]
use mockall::automock;

use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Response, StatusCode,
};
use secp256k1::PublicKey;

use crate::error::CashuWalletError;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Client {
    async fn post_split_tokens(
        &self,
        amount: u64,
        proofs: Proofs,
        output: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, CashuWalletError>;

    async fn post_mint_payment_request(
        &self,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, CashuWalletError>;

    async fn post_melt_tokens(
        &self,
        proofs: Proofs,
        pr: String,
    ) -> Result<PostMeltResponse, CashuWalletError>;

    async fn post_checkfees(&self, pr: String) -> Result<CheckFeesResponse, CashuWalletError>;

    async fn get_mint_keys(&self) -> Result<HashMap<u64, PublicKey>, CashuWalletError>;

    async fn get_mint_keysets(&self) -> Result<Keysets, CashuWalletError>;

    async fn get_mint_payment_request(
        &self,
        amount: u64,
    ) -> Result<PaymentRequest, CashuWalletError>;
}

#[derive(Debug, Clone)]
pub struct HttpClient {
    mint_url: String,
    request_client: reqwest::Client,
}

#[derive(serde::Deserialize, Debug)]
struct CashuErrorResponse {
    code: u64,
    error: String,
}

impl HttpClient {
    pub fn new(mint_url: String) -> Self {
        Self {
            mint_url,
            request_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Client for HttpClient {
    async fn post_split_tokens(
        &self,
        amount: u64,
        proofs: Proofs,
        output: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, CashuWalletError> {
        let url = format!("{}/split", self.mint_url);
        let body = serde_json::to_string(&PostSplitRequest {
            amount,
            proofs,
            outputs: output,
        })?;

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;

        extract_response_data::<PostSplitResponse>(resp).await
    }

    async fn post_melt_tokens(
        &self,
        proofs: Proofs,
        pr: String,
    ) -> Result<PostMeltResponse, CashuWalletError> {
        let url = format!("{}/melt", self.mint_url);
        let body = serde_json::to_string(&PostMeltRequest {
            pr,
            proofs,
            outputs: vec![],
        })?;

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;
        extract_response_data::<PostMeltResponse>(resp).await
    }

    async fn post_checkfees(&self, pr: String) -> Result<CheckFeesResponse, CashuWalletError> {
        let url = format!("{}/checkfees", self.mint_url);
        let body = serde_json::to_string(&CheckFeesRequest { pr })?;

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;

        extract_response_data::<CheckFeesResponse>(resp).await
    }

    async fn get_mint_keys(&self) -> Result<HashMap<u64, PublicKey>, CashuWalletError> {
        let url = format!("{}/keys", self.mint_url);
        let resp = self.request_client.get(url).send().await?;
        extract_response_data::<HashMap<u64, PublicKey>>(resp).await
    }

    async fn get_mint_keysets(&self) -> Result<Keysets, CashuWalletError> {
        let url = format!("{}/keysets", self.mint_url);
        let resp = self.request_client.get(url).send().await?;
        extract_response_data::<Keysets>(resp).await
    }

    async fn get_mint_payment_request(
        &self,
        amount: u64,
    ) -> Result<PaymentRequest, CashuWalletError> {
        let url = format!("{}/mint?amount={}", self.mint_url, amount);
        let resp = self.request_client.get(url).send().await?;
        extract_response_data::<PaymentRequest>(resp).await
    }

    async fn post_mint_payment_request(
        &self,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, CashuWalletError> {
        //        let url = format!("{}/mint?payment_hash={}", self.mint_url, hash); // TODO old query param
        let url = format!("{}/mint?hash={}", self.mint_url, hash);
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
) -> Result<T, CashuWalletError> {
    match response.status() {
        StatusCode::OK => {
            let response_text = response.text().await?;
            match serde_json::from_str::<T>(&response_text) {
                Ok(data) => Ok(data),
                Err(..) => Err(CashuWalletError::UnexpectedResponse(response_text)),
            }
        }
        _ => match &response.headers().get(CONTENT_TYPE) {
            Some(content_type) => {
                if *content_type == "application/json" {
                    let txt = response.text().await?;
                    let data = serde_json::from_str::<CashuErrorResponse>(&txt)
                        .map_err(|_| CashuWalletError::UnexpectedResponse(txt))
                        .unwrap();
                    Err(CashuWalletError::MintError(data.code, data.error))
                } else {
                    Err(CashuWalletError::UnexpectedResponse(response.text().await?))
                }
            }
            None => Err(CashuWalletError::UnexpectedResponse(response.text().await?)),
        },
    }
}
