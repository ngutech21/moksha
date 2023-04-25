use std::collections::HashMap;

use cashurs_core::model::{
    BlindedMessage, Keysets, PaymentRequest, PostMingRequest, PostMintResponse,
};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secp256k1::PublicKey;

use crate::error::CashuWalletError;

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

        Ok(resp.json::<PaymentRequest>().await?)
    }

    pub async fn post_mint_payment_request(
        &self,
        payment_hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, CashuWalletError> {
        let url = format!("{}/mint?payment_hash={}", self.mint_url, payment_hash);
        let body = serde_json::to_string(&PostMingRequest {
            outputs: blinded_messages,
        })?;

        println!("{}", &body.clone());

        let resp = self
            .request_client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(body)
            .send()
            .await?;
        let response = resp.text().await?;
        Ok(serde_json::from_str::<PostMintResponse>(&response)?)
    }
}
