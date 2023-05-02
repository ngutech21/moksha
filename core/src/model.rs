use base64::{engine::general_purpose, Engine as _};
use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{HashMap, HashSet};

use crate::{
    crypto::{derive_keys, derive_keyset_id, derive_pubkeys},
    error::CashuCoreError,
};

const TOKEN_PREFIX_V3: &str = "cashuA";

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

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Proof {
    pub amount: u64,
    pub secret: String,
    #[serde(rename = "C")]
    pub c: PublicKey,
    pub id: Option<String>,
    pub script: Option<P2SHScript>,
}

impl Proof {
    pub fn new(amount: u64, secret: String, c: PublicKey, id: String) -> Self {
        Self {
            amount,
            secret,
            c,
            id: Some(id),
            script: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2SHScript {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct Token {
    pub mint: Option<String>,
    pub proofs: Proofs,
}

// FIXME rename to TokenV3
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    #[serde(rename = "token")]
    pub tokens: Vec<Token>,
    pub memo: Option<String>,
}

impl Tokens {
    pub fn new(token: Token) -> Self {
        Self {
            tokens: vec![token],
            memo: None,
        }
    }

    // FIXME use From<Proofs> instead
    pub fn new_from_proofs(mint: String, proofs: Proofs) -> Self {
        Self {
            tokens: vec![Token {
                mint: Some(mint),
                proofs,
            }],
            memo: None,
        }
    }

    pub fn get_total_amount(&self) -> u64 {
        self.tokens
            .iter()
            .map(|token| {
                token
                    .proofs
                    .get_proofs()
                    .iter()
                    .map(|proof| proof.amount)
                    .sum::<u64>()
            })
            .sum()
    }

    pub fn get_proofs(&self) -> Proofs {
        Proofs::new(
            self.tokens
                .iter()
                .flat_map(|token| token.proofs.get_proofs())
                .collect(),
        )
    }

    pub fn serialize(&self) -> Result<String, CashuCoreError> {
        let json = serde_json::to_string(&self)?;
        Ok(format!(
            "{}{}",
            TOKEN_PREFIX_V3,
            general_purpose::URL_SAFE.encode(json.as_bytes())
        ))
    }

    pub fn deserialize(data: String) -> Result<Tokens, CashuCoreError> {
        if !data.starts_with(TOKEN_PREFIX_V3) {
            return Err(CashuCoreError::InvalidTokenPrefix);
        }

        let json = general_purpose::URL_SAFE.decode(
            data.strip_prefix(TOKEN_PREFIX_V3)
                .expect("Token does not contain prefix")
                .as_bytes(),
        )?;
        let token = serde_json::from_slice::<Tokens>(&json)?;
        Ok(token)
    }
}

impl From<(String, Proofs)> for Tokens {
    fn from(from: (String, Proofs)) -> Self {
        Self {
            tokens: vec![Token {
                mint: Some(from.0),
                proofs: from.1,
            }],
            memo: None,
        }
    }
}

pub trait TotalAmount {
    fn total_amount(&self) -> u64;
}

impl TotalAmount for Vec<BlindedSignature> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Proofs(pub(super) Vec<Proof>);

impl Proofs {
    pub fn new(proofs: Vec<Proof>) -> Self {
        Self(proofs)
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn get_total_amount(&self) -> u64 {
        self.0.iter().map(|proof| proof.amount).sum()
    }

    pub fn get_proofs(&self) -> Vec<Proof> {
        self.0.clone()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn has_duplicate_proofs(&self) -> bool {
        let secrets = self
            .get_proofs()
            .into_iter()
            .map(|x| x.secret)
            .collect::<Vec<String>>();
        secrets.len() != secrets.into_iter().collect::<HashSet<_>>().len()
    }
}

#[derive(Debug, Clone)]
pub struct MintKeyset {
    pub private_keys: HashMap<u64, SecretKey>,
    pub public_keys: HashMap<u64, PublicKey>,
    pub keyset_id: String,
}

impl MintKeyset {
    pub fn new(seed: String) -> MintKeyset {
        let priv_keys = derive_keys(&seed, "derivation_path"); // FIXME extract derivation path
        let pub_keys = derive_pubkeys(&priv_keys);
        MintKeyset {
            private_keys: priv_keys,
            keyset_id: derive_keyset_id(&pub_keys),
            public_keys: pub_keys,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keysets {
    pub keysets: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub pr: String,
    pub hash: String, // TODO use sha256::Hash
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub fee: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMeltRequest {
    pub proofs: Proofs,
    pub pr: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMeltResponse {
    pub paid: bool,
    pub preimage: String,
    pub change: Vec<BlindedSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostSplitRequest {
    pub proofs: Proofs,
    pub outputs: Vec<BlindedMessage>,
    pub amount: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostSplitResponse {
    pub fst: Vec<BlindedSignature>,
    pub snd: Vec<BlindedSignature>,
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

#[cfg(test)]
mod tests {
    use crate::{
        dhke,
        model::{Proof, Proofs, Token, Tokens},
    };
    use serde_json::json;

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let amount = 13;
        let bits = super::split_amount(amount);
        assert_eq!(bits, vec![1, 4, 8]);
        Ok(())
    }

    #[test]
    fn test_proof() -> anyhow::Result<()> {
        let js = json!(
            {
              "id": "DSAl9nvvyfva",
              "amount": 2,
              "secret": "EhpennC9qB3iFlW8FZ_pZw",
              "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
            }
        );

        let proof = serde_json::from_value::<Proof>(js)?;
        assert_eq!(proof.amount, 2);
        assert_eq!(proof.id, Some("DSAl9nvvyfva".to_string()));
        assert_eq!(proof.secret, "EhpennC9qB3iFlW8FZ_pZw".to_string());
        assert_eq!(
            proof.c.to_string(),
            "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4".to_string()
        );
        Ok(())
    }

    #[test]
    fn test_token() -> anyhow::Result<()> {
        let js = json!(
            {
              "mint": "https://8333.space:3338",
              "proofs": [
                {
                  "id": "DSAl9nvvyfva",
                  "amount": 2,
                  "secret": "EhpennC9qB3iFlW8FZ_pZw",
                  "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
                },
                {
                  "id": "DSAl9nvvyfva",
                  "amount": 8,
                  "secret": "TmS6Cv0YT5PU_5ATVKnukw",
                  "C": "02ac910bef28cbe5d7325415d5c263026f15f9b967a079ca9779ab6e5c2db133a7"
                }
              ]
        });

        let token = serde_json::from_value::<super::Token>(js)?;
        assert_eq!(token.mint, Some("https://8333.space:3338".to_string()));
        assert_eq!(token.proofs.len(), 2);
        Ok(())
    }

    #[test]
    fn test_tokens_serialize() -> anyhow::Result<()> {
        let token = Token {
            mint: Some("mymint".to_string()),
            proofs: Proofs::new(vec![Proof {
                amount: 21,
                secret: "secret".to_string(),
                c: dhke::public_key_from_hex(
                    "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
                ),
                id: None,
                script: None,
            }]),
        };
        let tokens = super::Tokens {
            tokens: vec![token],
            memo: Some("my memo".to_string()),
        };

        let serialized = tokens.serialize()?;
        dbg!(&serialized);
        assert!(serialized.starts_with("cashuA"));
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize() -> anyhow::Result<()> {
        let input = "cashuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJpZCI6IkRTQWw5bnZ2eWZ2YSIsImFtb3VudCI6Miwic2VjcmV0IjoiRWhwZW5uQzlxQjNpRmxXOEZaX3BadyIsIkMiOiIwMmMwMjAwNjdkYjcyN2Q1ODZiYzMxODNhZWNmOTdmY2I4MDBjM2Y0Y2M0NzU5ZjY5YzYyNmM5ZGI1ZDhmNWI1ZDQifSx7ImlkIjoiRFNBbDludnZ5ZnZhIiwiYW1vdW50Ijo4LCJzZWNyZXQiOiJUbVM2Q3YwWVQ1UFVfNUFUVktudWt3IiwiQyI6IjAyYWM5MTBiZWYyOGNiZTVkNzMyNTQxNWQ1YzI2MzAyNmYxNWY5Yjk2N2EwNzljYTk3NzlhYjZlNWMyZGIxMzNhNyJ9XX1dLCJtZW1vIjoiVGhhbmt5b3UuIn0=";
        let tokens = Tokens::deserialize(input.to_string())?;
        assert_eq!(tokens.memo, Some("Thankyou.".to_string()),);
        assert_eq!(tokens.tokens.len(), 1);
        Ok(())
    }
}
