//! This module defines the `Token` struct, which is used for representing tokens in Cashu as described in [Nut-00](https://github.com/cashubtc/nuts/blob/main/00.md)
//!
//! The `Token` struct represents a token, with an optional `mint` field for the URL of the Mint and a `proofs` field for the proofs associated with the token.

use std::str::FromStr;

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::skip_serializing_none;
use url::Url;

use crate::{error::MokshaCoreError, primitives::CurrencyUnit, proof::Proofs};

const TOKEN_PREFIX_V3: &str = "cashuA";

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    #[serde(serialize_with = "serialize_url", deserialize_with = "deserialize_url")]
    pub mint: Option<Url>,
    pub proofs: Proofs,
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str: Option<String> = Option::deserialize(deserializer)?;
    url_str.map_or_else(
        || Ok(None),
        |s| Url::parse(&s).map_err(serde::de::Error::custom).map(Some),
    )
}

fn serialize_url<S>(url: &Option<Url>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match url {
        Some(url) => {
            let mut url_str = url.as_str().to_owned();
            if url_str.ends_with('/') {
                url_str.pop();
            }
            serializer.serialize_str(&url_str)
        }
        None => serializer.serialize_none(),
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenV3 {
    #[serde(rename = "token")]
    pub tokens: Vec<Token>,
    pub unit: Option<CurrencyUnit>,
    pub memo: Option<String>,
}

impl TokenV3 {
    pub fn new(token: Token) -> Self {
        Self {
            tokens: vec![token],
            memo: None,
            unit: None,
        }
    }

    pub const fn empty() -> Self {
        Self {
            tokens: vec![],
            memo: None,
            unit: None,
        }
    }

    pub fn total_amount(&self) -> u64 {
        self.tokens
            .iter()
            .map(|token| {
                token
                    .proofs
                    .proofs()
                    .iter()
                    .map(|proof| proof.amount)
                    .sum::<u64>()
            })
            .sum()
    }

    pub fn proofs(&self) -> Proofs {
        Proofs::new(
            self.tokens
                .iter()
                .flat_map(|token| token.proofs.proofs())
                .collect(),
        )
    }

    pub fn serialize(&self) -> Result<String, MokshaCoreError> {
        let json = serde_json::to_string(&self)?;
        Ok(format!(
            "{}{}",
            TOKEN_PREFIX_V3,
            general_purpose::URL_SAFE.encode(json.as_bytes())
        ))
    }

    pub fn deserialize(data: impl Into<String>) -> Result<Self, MokshaCoreError> {
        let data = data.into();
        let token = data
            .strip_prefix(TOKEN_PREFIX_V3)
            .ok_or(MokshaCoreError::InvalidTokenPrefix)?;

        let json = general_purpose::URL_SAFE_NO_PAD
            .decode(token.as_bytes())
            .or_else(|_| general_purpose::URL_SAFE.decode(token.as_bytes()))
            .map_err(|_| MokshaCoreError::InvalidToken)?;

        Ok(serde_json::from_slice::<Self>(&json)?)
    }

    pub fn mint(&self) -> Option<Url> {
        self.tokens
            .first()
            .and_then(|token| token.mint.as_ref())
            .map(|url| url.to_owned())
    }
}

impl TryFrom<TokenV3> for String {
    type Error = MokshaCoreError;

    fn try_from(token: TokenV3) -> Result<Self, Self::Error> {
        token.serialize()
    }
}

impl TryFrom<String> for TokenV3 {
    type Error = MokshaCoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::deserialize(value)
    }
}

impl FromStr for TokenV3 {
    type Err = MokshaCoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::deserialize(s)
    }
}

impl From<(Url, Proofs)> for TokenV3 {
    fn from(from: (Url, Proofs)) -> Self {
        Self {
            tokens: vec![Token {
                mint: Some(from.0),
                proofs: from.1,
            }],
            memo: None,
            unit: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use url::Url;

    use crate::{
        dhke,
        fixture::read_fixture,
        primitives::CurrencyUnit,
        proof::Proof,
        token::{Token, TokenV3},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_token_v3() -> anyhow::Result<()> {
        let js = json!(
        {
          "token": [
            {
              "mint": "https://8333.space:3338",
              "proofs": [
                {
                  "amount": 2,
                  "id": "009a1f293253e41e",
                  "secret": "407915bc212be61a77e3e6d2aeb4c727980bda51cd06a6afc29e2861768a7837",
                  "C": "02bc9097997d81afb2cc7346b5e4345a9346bd2a506eb7958598a72f0cf85163ea"
                },
                {
                  "amount": 8,
                  "id": "009a1f293253e41e",
                  "secret": "fe15109314e61d7756b0f8ee0f23a624acaa3f4e042f61433c728c7057b931be",
                  "C": "029e8e5050b890a7d6c0968db16bc1d5d5fa040ea1de284f6ec69d61299f671059"
                }
              ]
            }
          ],
          "unit": "sat",
          "memo": "Thank you."
        });

        let token = serde_json::from_value::<super::TokenV3>(js)?;
        assert_eq!(
            token.tokens[0].mint,
            Some(Url::parse("https://8333.space:3338")?)
        );
        assert_eq!(token.tokens[0].proofs.len(), 2);
        assert_eq!(token.unit, Some(CurrencyUnit::Sat));

        let token_serialized = token.serialize()?;
        let fixture = read_fixture("token_nut_example.cashu")?;
        assert_eq!(token_serialized, fixture);
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
        assert_eq!(token.mint, Some(Url::parse("https://8333.space:3338")?));
        assert_eq!(token.proofs.len(), 2);
        Ok(())
    }

    #[test]
    fn test_tokens_serialize() -> anyhow::Result<()> {
        use base64::{engine::general_purpose, Engine as _};
        let token = Token {
            mint: Some(Url::parse("https://8333.space:3338/")?),
            proofs: Proof {
                amount: 21,
                secret: "secret".to_string(),
                c: dhke::public_key_from_hex(
                    "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
                ),
                keyset_id: "someid".to_string(),
                script: None,
            }
            .into(),
        };
        let tokens = super::TokenV3 {
            tokens: vec![token],
            memo: Some("my memo".to_string()),
            unit: None,
        };

        let serialized: String = tokens.try_into()?;
        assert!(serialized.starts_with("cashuA"));

        // check if mint is serialized without trailing slash
        let json = general_purpose::URL_SAFE.decode(serialized.strip_prefix("cashuA").unwrap())?;
        let deser = String::from_utf8(json)?;
        let json: Value = serde_json::from_str(&deser)?;
        let mint_value = json["token"][0]["mint"].as_str();
        assert_eq!(mint_value, Some("https://8333.space:3338"));
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize() -> anyhow::Result<()> {
        let input = read_fixture("token_nut_example.cashu")?;
        let tokens = TokenV3::deserialize(input)?;
        assert_eq!(tokens.memo, Some("Thank you.".to_string()),);
        assert_eq!(tokens.tokens.len(), 1);
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize_no_pad() -> anyhow::Result<()> {
        let input = read_fixture("token_no_pad60.cashu")?;
        let tokens = TokenV3::deserialize(input)?;
        assert_eq!(tokens.memo, None);
        assert_eq!(tokens.tokens.len(), 1);
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize_with_padding() -> anyhow::Result<()> {
        let input = read_fixture("token_60.cashu")?;
        let tokens = TokenV3::deserialize(input)?;
        assert_eq!(tokens.tokens.len(), 1);
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize_invalid() -> anyhow::Result<()> {
        let input = read_fixture("token_invalid.cashu")?;
        let tokens = TokenV3::deserialize(input);
        assert!(tokens.is_err());
        Ok(())
    }
}
