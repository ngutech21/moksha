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
    client: Box<dyn Client>,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
    dhke: Dhke,
}

impl Wallet {
    pub fn new(
        client: Box<dyn Client>,
        mint_keys: HashMap<u64, PublicKey>,
        keysets: Keysets,
    ) -> Self {
        Self {
            client,
            mint_keys,
            keysets,
            dhke: Dhke::new(),
        }
    }

    pub async fn split_tokens(
        &self,
        tokens: Tokens,
        splt_amount: u64,
        mint_url: String,
    ) -> Result<(Tokens, Tokens), CashuWalletError> {
        let total_token_amount = tokens.get_total_amount();
        let first_secrets = self.create_secrets(&split_amount(splt_amount));
        let first_outputs = self.create_blinded_messages(splt_amount, first_secrets.clone())?;

        println!("First outputs: {:?}", first_outputs);

        // ############################################################################

        let second_amount = total_token_amount - splt_amount;
        let second_secrets = self.create_secrets(&split_amount(second_amount));
        let second_outputs = self.create_blinded_messages(second_amount, second_secrets.clone())?;

        println!("Second outputs: {:?}", second_outputs);

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        println!("total outputs: {:?}", total_outputs);

        let sum_proofs = tokens.get_total_amount();

        println!("Sum proofs: {:?}", sum_proofs);

        let split_result = self
            .client
            .post_split_tokens(splt_amount, tokens.get_proofs(), total_outputs)
            .await?;

        println!(
            ">> Split result: {}",
            serde_json::to_string(&split_result).unwrap()
        );

        let first_tokens = Tokens::from((
            mint_url.clone(),
            self.create_proofs_from_blinded_signatures(
                split_result.fst,
                first_secrets,
                first_outputs,
            )?,
        ));

        let second_tokens = Tokens::from((
            mint_url.clone(),
            self.create_proofs_from_blinded_signatures(
                split_result.snd,
                second_secrets,
                second_outputs,
            )?,
        ));

        Ok((first_tokens, second_tokens))
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

        println!("split_amount {:?} total {:?}", split_amount, amount);

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

fn get_blinded_msg(blinded_messages: Vec<(BlindedMessage, SecretKey)>) -> Vec<BlindedMessage> {
    blinded_messages
        .into_iter()
        .map(|(msg, _)| msg)
        .collect::<Vec<BlindedMessage>>()
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Wallet;
    use crate::client::{HttpClient, MockClient};
    use cashurs_core::model::{Keysets, PostSplitResponse, Tokens};
    use std::collections::HashMap;

    #[test]
    fn test_create_secrets() {
        let client = HttpClient::new("http://localhost:8080".to_string());
        let wallet = Wallet::new(
            Box::new(client),
            HashMap::new(),
            Keysets { keysets: vec![] },
        );

        let amounts = vec![1, 2, 3, 4, 5, 6, 7];
        let secrets = wallet.create_secrets(&amounts);

        assert!(secrets.len() == amounts.len());
    }

    #[tokio::test]
    async fn test_split() -> anyhow::Result<()> {
        let mut client = MockClient::new();

        client.expect_post_split_tokens().returning(|_, _, _| {
            Ok(PostSplitResponse {
                fst: vec![],
                snd: vec![],
            })
        });

        let wallet = Wallet::new(
            Box::new(client),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
        );

        let raw_token = "cashuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHA6Ly8xMjcuMC4wLjE6MzMzOCIsInByb29mcyI6W3siYW1vdW50Ijo2NCwic2VjcmV0IjoibXpkdzJFRUszOGptSXdGQ0x6OWJISGZEIiwiQyI6IjAzNGRiOTU2Zjg0OTE3ZGRhMmRhMDgzNTc2OGFkZTUzOWFjMzhjZjA0MmZhYWY4NDk3NTJjNWE3N2I5YmIwOGQ2ZCIsImlkIjoicGFGYk8xNDJfc3VpIn1dfV19";
        let tokens = Tokens::deserialize(raw_token.to_string())?;

        let result = wallet
            .split_tokens(tokens, 20, "mint_url".to_string())
            .await;

        println!("{:?}", result);

        Ok(())
    }

    #[test]
    fn test_splitup() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let fst = &items[0..3];
        let snd = &items[3..5];

        println!("{:?} {:?}", fst, snd);
    }
}
