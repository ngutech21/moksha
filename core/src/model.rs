use base64::{engine::general_purpose, Engine as _};
use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self},
};

use crate::crypto::{derive_keys, derive_keyset_id, derive_pubkeys};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    pub b_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSignature {
    pub amount: u64,
    #[serde(rename = "C_")]
    pub c_: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub amount: u64,
    pub secret: String,
    #[serde(rename = "C")]
    pub c: String,
    pub id: Option<String>,
    pub script: Option<P2SHScript>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2SHScript {}

const TOKEN_PREFIX_V3: &str = "cashuA";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    mint: Option<String>,
    proofs: Proofs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    #[serde(rename = "token")]
    pub tokens: Vec<Token>,
    pub memo: Option<String>,
}

impl Tokens {
    pub fn serialize(&self) -> io::Result<String> {
        let json = serde_json::to_string(&self)?;
        Ok(format!(
            "{}{}",
            TOKEN_PREFIX_V3,
            general_purpose::URL_SAFE.encode(json.as_bytes())
        ))
    }

    pub fn deserialize(data: String) -> io::Result<Tokens> {
        if !data.starts_with(TOKEN_PREFIX_V3) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid token prefix",
            ));
        }

        let json = general_purpose::URL_SAFE
            .decode(
                data.strip_prefix(TOKEN_PREFIX_V3)
                    .expect("Token does not contain prefix")
                    .as_bytes(),
            )
            .unwrap(); // FIXME: handle error
        let token = serde_json::from_slice::<Tokens>(&json)?;
        Ok(token)
    }
}

pub type Proofs = Vec<Proof>;

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

#[cfg(test)]
mod tests {
    use crate::model::{Proof, Token, Tokens};
    use serde_json::json;

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
            proof.c,
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
            proofs: vec![Proof {
                amount: 21,
                secret: "secret".to_string(),
                c: "c".to_string(),
                id: None,
                script: None,
            }],
        };
        let tokens = super::Tokens {
            tokens: vec![token],
            memo: Some("my memo".to_string()),
        };

        let serialized = tokens.serialize()?;
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
