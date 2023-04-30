use std::collections::HashMap;

use cashurs_core::{
    dhke::Dhke,
    model::{
        split_amount, BlindedMessage, BlindedSignature, Keysets, PostMeltResponse, Proof, Proofs,
        Tokens,
    },
};
use secp256k1::{PublicKey, SecretKey};

use crate::{client::Client, error::CashuWalletError};
use rand::{distributions::Alphanumeric, Rng};

pub struct Wallet {
    client: Client,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
    dhke: Dhke,
}

impl Wallet {
    pub fn new(client: Client, mint_keys: HashMap<u64, PublicKey>, keysets: Keysets) -> Self {
        Self {
            client,
            mint_keys,
            keysets,
            dhke: Dhke::new(),
        }
    }

    pub async fn melt_token(
        &self,
        pr: String,
        tokens: Tokens,
    ) -> Result<PostMeltResponse, CashuWalletError> {
        let _fees = self.client.post_checkfees(pr.clone()).await.unwrap();
        // TODO get tokens for fee amount
        let proofs = tokens.get_proofs();
        let melt_response = self.client.post_melt_tokens(proofs, pr).await?;
        Ok(melt_response)
    }

    pub fn create_secrets(&self, split_amount: &Vec<u64>) -> Vec<String> {
        (0..split_amount.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }

    pub async fn mint_tokens(&self, amount: u64, hash: String) -> Result<Proofs, CashuWalletError> {
        let splited_amount = split_amount(amount);
        let secrets = self.create_secrets(&splited_amount);

        let blinded_messages = splited_amount
            .into_iter()
            .zip(secrets.clone())
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>();

        let post_mint_resp = self
            .client
            .post_mint_payment_request(
                hash,
                blinded_messages
                    .clone()
                    .into_iter()
                    .map(|(msg, _)| msg)
                    .collect::<Vec<BlindedMessage>>(),
            )
            .await?;

        // step 3: unblind signatures
        let keysets = &self.keysets.keysets;
        let current_keyset = keysets[keysets.len() - 1].clone();

        let private_keys = blinded_messages
            .clone()
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        Ok(Proofs::new(
            post_mint_resp
                .promises
                .iter()
                .zip(private_keys)
                .zip(secrets)
                .map(|((p, priv_key), secret)| {
                    let key = self
                        .mint_keys
                        .get(&p.amount)
                        .expect("msg amount not found in mint keys");
                    let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                    Proof::new(p.amount, secret, pub_alice, current_keyset.clone())
                })
                .collect::<Vec<Proof>>(),
        ))
    }

    pub fn create_blinded_messages(
        &self,
        amount: u64,
        secrets: Vec<String>,
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, CashuWalletError> {
        let split_amount = split_amount(amount);

        Ok(split_amount
            .into_iter()
            .zip(secrets)
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>())
    }

    pub fn create_proofs_from_blinded_signatures(
        &self,
        signatures: Vec<BlindedSignature>,
        secrets: Vec<String>,
        outputs: Vec<(BlindedMessage, SecretKey)>,
    ) -> Result<Proofs, CashuWalletError> {
        let keysets = &self.keysets.keysets;
        let current_keyset = keysets[keysets.len() - 1].clone();

        let private_keys = outputs
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        Ok(Proofs::new(
            signatures
                .iter()
                .zip(private_keys)
                .zip(secrets)
                .map(|((p, priv_key), secret)| {
                    let key = self
                        .mint_keys
                        .get(&p.amount)
                        .expect("msg amount not found in mint keys");
                    let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                    Proof::new(p.amount, secret, pub_alice, current_keyset.clone())
                })
                .collect::<Vec<Proof>>(),
        ))
    }
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}
