//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display};

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::convert::TryFrom;
use utoipa::ToSchema;
use uuid::Uuid;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostSwapRequest {
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostSwapResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Debug)]
pub struct CashuErrorResponse {
    pub code: u64,
    pub detail: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct MintLegacyInfoResponse {
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Default, ToSchema)]
pub struct KeysResponse {
    pub keysets: Vec<KeyResponse>,
}

#[derive(serde::Deserialize, Serialize, Debug, PartialEq, Eq, ToSchema)]
pub struct KeyResponse {
    pub id: String,
    pub unit: CurrencyUnit,
    #[schema(value_type = HashMap<u64, String>)]
    pub keys: HashMap<u64, PublicKey>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CurrencyUnit {
    Sat,
    Usd,
}

impl Display for CurrencyUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrencyUnit::Sat => write!(f, "sat"),
            CurrencyUnit::Usd => write!(f, "usd"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Bolt11,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintQuoteBolt11Request {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMintQuoteBolt11Response {
    pub quote: String,
    #[serde(rename = "request")]
    pub payment_request: String,
    pub paid: bool,
    pub expiry: u64,
}
impl TryFrom<Quote> for PostMintQuoteBolt11Response {
    type Error = &'static str;

    fn try_from(quote: Quote) -> Result<Self, Self::Error> {
        match quote {
            Quote::Bolt11Mint {
                quote_id,
                payment_request,
                expiry,
                paid,
            } => Ok(Self {
                quote: quote_id.to_string(),
                payment_request,
                expiry,
                paid,
            }),
            _ => Err("Invalid quote variant"),
        }
    }
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
    pub paid: bool,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Quote {
    Bolt11Mint {
        quote_id: Uuid,
        payment_request: String,
        expiry: u64,
        paid: bool,
    },
    Bolt11Melt {
        quote_id: Uuid,
        amount: u64,
        fee_reserve: u64,
        payment_request: String,
        expiry: u64,
        paid: bool,
    },
}
impl TryFrom<Quote> for PostMeltQuoteBolt11Response {
    type Error = &'static str;

    fn try_from(quote: Quote) -> Result<Self, Self::Error> {
        match quote {
            Quote::Bolt11Melt {
                quote_id,
                amount,
                fee_reserve,
                expiry,
                paid,
                ..
            } => Ok(Self {
                quote: quote_id.to_string(),
                amount,
                fee_reserve,
                paid,
                expiry,
            }),
            _ => Err("Invalid quote variant"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltBolt11Request {
    pub quote: String,
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMeltBolt11Response {
    pub paid: bool,
    pub payment_preimage: String,
    pub change: Vec<BlindedSignature>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, ToSchema)]
pub struct MintInfoResponse {
    pub name: Option<String>,
    #[schema(value_type = String)]
    pub pubkey: PublicKey,
    pub version: Option<String>,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
    pub nuts: Vec<MintInfoNut>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub enum MintInfoNut {
    /// Cryptography and Models
    #[serde(rename = "0")]
    Nut0 { disabled: bool },

    /// Mint public keys
    #[serde(rename = "1")]
    Nut1 { disabled: bool },

    /// Keysets and keyset IDs
    #[serde(rename = "2")]
    Nut2 { disabled: bool },

    /// Swapping tokens
    #[serde(rename = "3")]
    Nut3 { disabled: bool },

    /// Minting tokens
    #[serde(rename = "4")]
    Nut4 {
        methods: Vec<(PaymentMethod, CurrencyUnit)>,
        disabled: bool,
    },

    /// Melting tokens
    #[serde(rename = "5")]
    Nut5 {
        methods: Vec<(PaymentMethod, CurrencyUnit)>,
        disabled: bool,
    },

    /// Mint info
    #[serde(rename = "6")]
    Nut6 { disabled: bool },

    /// Token state check
    #[serde(rename = "7")]
    Nut7 { supported: bool },

    /// Overpaid Lightning fees
    #[serde(rename = "8")]
    Nut8 { supported: bool },

    /// Deterministic backup and restore
    #[serde(rename = "9")]
    Nut9 { supported: bool },

    /// Spending conditions
    #[serde(rename = "10")]
    Nut10 { supported: bool },

    /// Pay-To-Pubkey (P2PK)
    #[serde(rename = "11")]
    Nut11 { supported: bool },

    /// DLEQ proofs
    #[serde(rename = "12")]
    Nut12 { supported: bool },
}

#[cfg(test)]
mod tests {

    use crate::{
        dhke::public_key_from_hex,
        primitives::{
            CurrencyUnit, KeyResponse, MintInfoNut, MintInfoResponse, MintLegacyInfoResponse,
            Parameter, PaymentMethod, PostSwapResponse,
        },
    };

    #[test]
    fn test_serialize_empty_swap_response() -> anyhow::Result<()> {
        let response = PostSwapResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"signatures\":[]}");
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
    fn test_deserialize_legacy_mint_info() -> anyhow::Result<()> {
        let mint_info = MintLegacyInfoResponse {
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
                MintInfoNut::Nut0 { disabled: false },
                MintInfoNut::Nut1 { disabled: false },
                MintInfoNut::Nut2 { disabled: false },
                MintInfoNut::Nut3 { disabled: false },
                MintInfoNut::Nut4 {
                    methods: vec![(PaymentMethod::Bolt11, CurrencyUnit::Sat)],
                    disabled: false,
                },
                MintInfoNut::Nut5 {
                    methods: vec![(PaymentMethod::Bolt11, CurrencyUnit::Sat)],
                    disabled: false,
                },
                MintInfoNut::Nut6 { disabled: false },
                MintInfoNut::Nut7 { supported: false },
                MintInfoNut::Nut8 { supported: false },
                MintInfoNut::Nut9 { supported: false },
                MintInfoNut::Nut10 { supported: false },
                MintInfoNut::Nut11 { supported: false },
                MintInfoNut::Nut12 { supported: false },
            ],
            motd: Some("Message to display to users.".to_string()),
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        println!("{}", out);
        assert!(!out.is_empty());
        // FIXME add asserts

        Ok(())
    }
}
