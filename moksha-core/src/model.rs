use rand::distributions::Alphanumeric;
use rand::Rng;
use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    crypto::{self, derive_keys, derive_keyset_id, derive_pubkey, derive_pubkeys},
    error::MokshaCoreError,
    proof::Proofs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    pub b_: PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSignature {
    pub amount: u64,
    #[serde(rename = "C_")]
    pub c_: PublicKey,
    pub id: Option<String>,
}

pub trait TotalAmount {
    fn total_amount(&self) -> u64;
}

impl TotalAmount for Vec<BlindedSignature> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}

impl TotalAmount for Vec<BlindedMessage> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}

#[derive(Debug, Clone)]
pub struct MintKeyset {
    pub private_keys: HashMap<u64, SecretKey>,
    pub public_keys: HashMap<u64, PublicKey>,
    pub keyset_id: String,
    pub mint_pubkey: PublicKey,
}

impl MintKeyset {
    pub fn new(seed: String, derivation_path: String) -> MintKeyset {
        let priv_keys = derive_keys(&seed, &derivation_path);
        let pub_keys = derive_pubkeys(&priv_keys);
        MintKeyset {
            private_keys: priv_keys,
            keyset_id: derive_keyset_id(&pub_keys),
            public_keys: pub_keys,
            mint_pubkey: derive_pubkey(&seed).expect("invalid seed"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Keysets {
    pub keysets: Vec<String>,
}

impl Keysets {
    pub fn new(keysets: Vec<String>) -> Self {
        Self { keysets }
    }

    pub fn get_current_keyset(
        &self,
        mint_keys: &HashMap<u64, PublicKey>,
    ) -> Result<String, MokshaCoreError> {
        let computed_id = crypto::derive_keyset_id(mint_keys);
        if self.keysets.contains(&computed_id) {
            Ok(computed_id)
        } else {
            Err(MokshaCoreError::InvalidKeysetid)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub pr: String,
    pub hash: String, // TODO use sha256::Hash
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostMintResponse {
    pub promises: Vec<BlindedSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMintRequest {
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesRequest {
    pub pr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesResponse {
    /// fee in satoshis
    pub fee: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMeltRequest {
    pub proofs: Proofs,
    pub pr: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostMeltResponse {
    pub paid: bool,
    pub preimage: String,
    pub change: Vec<BlindedSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostSplitRequest {
    pub proofs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostSplitResponse {
    pub promises: Vec<BlindedSignature>,
}

impl PostSplitResponse {
    pub fn with_promises(promises: Vec<BlindedSignature>) -> Self {
        Self { promises }
    }
}

#[derive(Clone)]
pub struct Amount(pub u64);

impl Amount {
    pub fn split(&self) -> SplitAmount {
        split_amount(self.0).into()
    }
}

#[derive(Clone)]
pub struct SplitAmount(pub Vec<u64>);

impl From<Vec<u64>> for SplitAmount {
    fn from(from: Vec<u64>) -> Self {
        Self(from)
    }
}

impl SplitAmount {
    pub fn create_secrets(&self) -> Vec<String> {
        (0..self.0.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }
}

impl From<u64> for Amount {
    fn from(amount: u64) -> Self {
        Self(amount)
    }
}

/// split a decimal amount into a vector of powers of 2
pub fn split_amount(amount: u64) -> Vec<u64> {
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

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[derive(serde::Deserialize, Debug)]
pub struct CashuErrorResponse {
    pub code: u64,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use crate::model::{PostSplitResponse, SplitAmount};

    #[test]
    fn test_serialize_empty_split_response() -> anyhow::Result<()> {
        let response = PostSplitResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"promises\":[]}");
        Ok(())
    }

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let bits = super::split_amount(13);
        assert_eq!(bits, vec![1, 4, 8]);

        let bits = super::split_amount(63);
        assert_eq!(bits, vec![1, 2, 4, 8, 16, 32]);

        let bits = super::split_amount(64);
        assert_eq!(bits, vec![64]);
        Ok(())
    }

    #[test]
    fn test_create_secrets() {
        let amounts = vec![1, 2, 3, 4, 5, 6, 7];
        let secrets = SplitAmount::from(amounts.clone()).create_secrets();
        assert!(secrets.len() == amounts.len());
        assert_eq!(secrets.get(0).unwrap().len(), 24);
    }
}
