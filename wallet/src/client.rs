use std::collections::HashMap;

use cashurs_core::model::{BlindedMessage, Keysets, PaymentRequest, PostMintResponse};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secp256k1::PublicKey;

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

    pub async fn get_mint_keys(&self) -> Result<HashMap<u64, PublicKey>, ()> {
        let url = format!("{}/keys", self.mint_url);
        let resp = self.request_client.get(url).send().await.unwrap();

        Ok(resp.json::<HashMap<u64, PublicKey>>().await.unwrap())
    }

    pub async fn get_mint_keysets(&self) -> Result<Keysets, ()> {
        let url = format!("{}/keysets", self.mint_url);
        let resp = self.request_client.get(url).send().await.unwrap();

        Ok(resp.json::<Keysets>().await.unwrap())
    }

    pub async fn get_mint_payment_request(&self, amount: u64) -> Result<PaymentRequest, ()> {
        let url = format!("{}/mint?amount={}", self.mint_url, amount);
        let resp = self.request_client.get(url).send().await.unwrap();

        Ok(resp.json::<PaymentRequest>().await.unwrap())
    }

    pub async fn post_mint_payment_request(
        &self,
        payment_hash: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, ()> {
        let url = format!("{}/mint?payment_hash={}", self.mint_url, payment_hash);
        //let url = format!("{}/mint", self.mint_url);

        let body = serde_json::to_string(&blinded_messages).unwrap();
        dbg!(body.clone());

        let resp = self
            .request_client
            .post(url)
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str("application/json").unwrap(),
            )
            .body(body)
            .send()
            .await
            .unwrap();

        Ok(resp.json::<PostMintResponse>().await.unwrap())
    }
}
