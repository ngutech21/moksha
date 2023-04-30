use std::collections::HashMap;

use cashurs_core::model::{
    BlindedMessage, CheckFeesRequest, CheckFeesResponse, Keysets, PaymentRequest, PostMeltRequest,
    PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
    Proofs,
};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secp256k1::PublicKey;

use crate::error::CashuWalletError;

#[derive(Debug, Clone)]
pub struct Client {
    mint_url: String,
    request_client: reqwest::Client,
}

impl Client {
    pub fn new(mint_url: String) -> Self {
        Self {
            mint_url,
            request_client: reqwest::Client::new(),
        }
    }

    pub async fn post_split_tokens(
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
        let response = resp.text().await?;

        // FIXME check for error
        Ok(serde_json::from_str::<PostSplitResponse>(&response)?)
    }

    pub async fn post_melt_tokens(
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
        let response = resp.text().await?;
        // FIXME check for error
        Ok(serde_json::from_str::<PostMeltResponse>(&response)?)
    }

    pub async fn post_checkfees(&self, pr: String) -> Result<CheckFeesResponse, CashuWalletError> {
        let url = format!("{}/checkfees", self.mint_url);
        let body = serde_json::to_string(&CheckFeesRequest { pr })?;

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;
        let response = resp.text().await?;
        Ok(serde_json::from_str::<CheckFeesResponse>(&response)?)
    }

    pub async fn get_mint_keys(&self) -> Result<HashMap<u64, PublicKey>, CashuWalletError> {
        let url = format!("{}/keys", self.mint_url);
        let resp = self.request_client.get(url).send().await?;

        Ok(resp.json::<HashMap<u64, PublicKey>>().await?)
    }

    pub async fn get_mint_keysets(&self) -> Result<Keysets, CashuWalletError> {
        let url = format!("{}/keysets", self.mint_url);
        let resp = self.request_client.get(url).send().await?;

        Ok(resp.json::<Keysets>().await?)
    }

    pub async fn get_mint_payment_request(
        &self,
        amount: u64,
    ) -> Result<PaymentRequest, CashuWalletError> {
        let url = format!("{}/mint?amount={}", self.mint_url, amount);
        let resp = self.request_client.get(url).send().await?;

        let response = resp.text().await?;
        println!("response: {}", &response);
        Ok(serde_json::from_str::<PaymentRequest>(&response)?)
    }

    pub async fn post_mint_payment_request(
        &self,
        hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, CashuWalletError> {
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
        let response = resp.text().await?;
        println!("response: {}", &response);
        Ok(serde_json::from_str::<PostMintResponse>(&response)?)
    }
}
