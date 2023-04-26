use std::collections::HashMap;

use cashurs_core::{
    dhke,
    model::{BlindedMessage, Keysets, PostMeltResponse, Proof, Tokens},
};
use secp256k1::{PublicKey, SecretKey};

use crate::{client::Client, error::CashuWalletError};
use rand::{distributions::Alphanumeric, Rng};

pub struct Wallet {
    client: Client,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
}

impl Wallet {
    pub fn new(client: Client, mint_keys: HashMap<u64, PublicKey>, keysets: Keysets) -> Self {
        Self {
            client,
            mint_keys,
            keysets,
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

    pub async fn mint_tokens(
        &self,
        amount: u64,
        payment_hash: String,
    ) -> Result<Vec<Proof>, CashuWalletError> {
        let split_amount = split_amount(amount);

        let secrets = (0..split_amount.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>();

        let blinded_messages = split_amount
            .into_iter()
            .zip(secrets.clone())
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = dhke::step1_alice(secret, None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>();

        let post_mint_resp = self
            .client
            .post_mint_payment_request(
                payment_hash,
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

        let result: Vec<Proof> = post_mint_resp
            .promises
            .iter()
            .zip(private_keys)
            .zip(secrets)
            .map(|((p, priv_key), secret)| {
                let key = self
                    .mint_keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = dhke::step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset.clone())
            })
            .collect::<Vec<Proof>>();
        Ok(result)
    }
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

/// split a decimal amount into a vector of powers of 2
fn split_amount(amount: u64) -> Vec<u64> {
    format!("{amount:b}")
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '1' {
                return Some(2_u64.pow(i as u32));
            }
            None
        })
        .collect::<Vec<u64>>()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let amount = 13;
        let bits = super::split_amount(amount);
        assert_eq!(bits, vec![1, 4, 8]);
        Ok(())
    }
}
