//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display};

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    blind::{BlindedMessage, BlindedSignature},
    proof::Proofs,
};

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

#[derive(Deserialize, Debug)]
pub struct CashuErrorResponse {
    pub code: u64,
    pub error: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct MintInfoResponse {
    pub name: Option<String>,
    pub pubkey: PublicKey,
    pub version: Option<String>,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub nuts: Vec<String>,
    pub motd: Option<String>,
    pub parameter: Parameter,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Default)]
pub struct Parameter {
    pub peg_out_only: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Default)]
pub struct KeysResponse {
    pub keysets: Vec<KeyResponse>,
}

#[derive(serde::Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct KeyResponse {
    pub id: String,
    pub unit: CurrencyUnit,
    pub keys: HashMap<u64, PublicKey>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub enum CurrencyUnit {
    #[serde(rename = "sat")]
    Sat,
}

impl Display for CurrencyUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrencyUnit::Sat => write!(f, "sat"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintQuoteBolt11Request {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintQuoteBolt11Response {
    pub quote: String,
    pub request: String,
    pub paid: bool,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintBolt11Request {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintBolt11Response {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltQuoteBolt11Request {
    /// payment request
    pub request: String,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltQuoteBolt11Response {
    pub quote: String,
    pub amount: u64,
    pub fee_reserve: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltBolt11Request {
    pub quote: String,
    pub inputs: Proofs,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltBolt11Response {
    pub paid: bool,
    pub payment_preimage: String,
    pub change: Vec<BlindedSignature>,
}

#[cfg(test)]
mod tests {
    use crate::{
        dhke::public_key_from_hex,
        primitives::{KeyResponse, MintInfoResponse, Parameter, PostSplitResponse},
    };

    #[test]
    fn test_serialize_empty_split_response() -> anyhow::Result<()> {
        let response = PostSplitResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"promises\":[]}");
        Ok(())
    }

    #[test]
    fn test_serialize_keyresponse() -> anyhow::Result<()> {
        let response = KeyResponse {
            id: "test".to_string(),
            unit: crate::primitives::CurrencyUnit::Sat,
            keys: std::collections::HashMap::new(),
        };
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"id\":\"test\",\"unit\":\"sat\",\"keys\":{}}");
        Ok(())
    }

    #[test]
    fn test_deserialize_mint_info() -> anyhow::Result<()> {
        let mint_info = MintInfoResponse {
            name: Some("Bob's Cashu mint".to_string()),
            pubkey: public_key_from_hex(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
            ),
            version: Some("Nutshell/0.11.0".to_string()),
            description: Some("The short mint description".to_string()),
            description_long: Some("A description that can be a long piece of text.".to_string()),
            contact: Some(vec![
                vec!["email".to_string(), "contact@me.com".to_string()],
                vec!["twitter".to_string(), "@me".to_string()],
                vec!["nostr".to_string(), "npub...".to_string()],
            ]),
            nuts: vec![
                "NUT-07".to_string(),
                "NUT-08".to_string(),
                "NUT-08".to_string(),
            ],
            motd: Some("Message to display to users.".to_string()),
            parameter: Parameter {
                peg_out_only: false,
            },
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        println!("{}", out);
        assert!(!out.is_empty());

        Ok(())
    }
}
