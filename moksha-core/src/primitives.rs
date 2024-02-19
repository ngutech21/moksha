//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display};

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostSwapRequest {
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct KeysResponse {
    pub keysets: Vec<KeyResponse>,
}

impl KeysResponse {
    pub fn new(keyset: KeyResponse) -> Self {
        Self {
            keysets: vec![keyset],
        }
    }
}

#[derive(serde::Deserialize, Serialize, Clone, Debug, PartialEq, Eq, ToSchema)]
pub struct KeyResponse {
    pub id: String, // TODO use new type for keyset_id
    pub unit: CurrencyUnit,
    #[schema(value_type = HashMap<u64, String>)]
    pub keys: HashMap<u64, PublicKey>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CurrencyUnit {
    Sat,
    Usd,
}

impl Display for CurrencyUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sat => write!(f, "sat"),
            Self::Usd => write!(f, "usd"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Bolt11,
    BtcOnchain,
}

impl Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bolt11 => write!(f, "Lightning"),
            Self::BtcOnchain => write!(f, "Onchain"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBolt11Request {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBolt11Response {
    pub quote: String,
    #[serde(rename = "request")]
    pub payment_request: String,
    pub paid: bool,
    pub expiry: Option<u64>,
}

impl From<Bolt11MintQuote> for PostMintQuoteBolt11Response {
    fn from(quote: Bolt11MintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            payment_request: quote.payment_request,
            paid: quote.paid,
            expiry: Some(quote.expiry),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBolt11Request {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBolt11Response {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBolt11Request {
    /// payment request
    pub request: String,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBolt11Response {
    pub quote: String,
    pub amount: u64,
    pub fee_reserve: u64,
    pub paid: bool,
    pub expiry: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bolt11MintQuote {
    pub quote_id: Uuid,
    pub payment_request: String,
    pub expiry: u64,
    pub paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bolt11MeltQuote {
    pub quote_id: Uuid,
    pub amount: u64,
    pub fee_reserve: u64,
    pub payment_request: String,
    pub expiry: u64,
    pub paid: bool,
}

impl From<Bolt11MeltQuote> for PostMeltQuoteBolt11Response {
    fn from(quote: Bolt11MeltQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            amount: quote.amount,
            fee_reserve: quote.fee_reserve,
            expiry: Some(quote.expiry),
            paid: quote.paid,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBolt11Request {
    pub quote: String,
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBolt11Response {
    pub paid: bool,
    pub payment_preimage: Option<String>,
    pub change: Vec<BlindedSignature>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct MintInfoResponse {
    pub name: Option<String>,
    #[schema(value_type = String)]
    pub pubkey: PublicKey,
    pub version: Option<String>,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
    pub nuts: Nuts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnchainMintQuote {
    pub quote_id: Uuid,
    pub address: String,
    pub unit: CurrencyUnit,
    pub amount: u64,
    pub expiry: u64,
    pub paid: bool,
}

impl From<OnchainMintQuote> for PostMintQuoteOnchainResponse {
    fn from(quote: OnchainMintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            address: quote.address,
            paid: quote.paid,
            expiry: quote.expiry,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnchainMeltQuote {
    pub quote_id: Uuid,
    pub amount: u64,
    pub address: String,
    pub fee_total: u64,
    pub fee_sat_per_vbyte: u32,
    pub expiry: u64,
    pub paid: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteOnchainRequest {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteOnchainResponse {
    pub quote: String,
    pub address: String,
    pub paid: bool,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintOnchainRequest {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintOnchainResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteOnchainRequest {
    pub amount: u64,
    /// onchain address
    pub address: String,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteOnchainResponse {
    pub quote: String,
    pub description: String,
    pub amount: u64,
    pub fee: u64,
    pub paid: bool,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltOnchainRequest {
    pub quote: String,
    pub inputs: Proofs,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltOnchainResponse {
    pub paid: bool,
    pub txid: String,
}

impl From<(String, OnchainMeltQuote)> for PostMeltQuoteOnchainResponse {
    fn from((description, quote): (String, OnchainMeltQuote)) -> Self {
        Self {
            description,
            quote: quote.quote_id.to_string(),
            amount: quote.amount,
            fee: quote.fee_total,
            expiry: quote.expiry,
            paid: quote.paid,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct GetMeltOnchainResponse {
    pub paid: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nuts {
    /// Minting tokens
    #[serde(rename = "4")]
    pub nut4: Nut4,

    /// Melting tokens
    #[serde(rename = "5")]
    pub nut5: Nut5,

    /// Token state check
    #[serde(rename = "7")]
    pub nut7: Nut7,

    /// Overpaid Lightning fees
    #[serde(rename = "8")]
    pub nut8: Nut8,

    /// Deterministic backup and restore
    #[serde(rename = "9")]
    pub nut9: Nut9,

    /// Spending conditions
    #[serde(rename = "10")]
    pub nut10: Nut10,

    /// Pay-To-Pubkey (P2PK)
    #[serde(rename = "11")]
    pub nut11: Nut11,

    #[serde(rename = "12")]
    /// DLEQ proofs
    pub nut12: Nut12,

    // TODO remove this if nut-14 and nut-15 are merged
    #[serde(rename = "14", skip_serializing_if = "Option::is_none")]
    /// minting tokens onchain
    pub nut14: Option<Nut14>,

    #[serde(rename = "15", skip_serializing_if = "Option::is_none")]
    /// melting tokens onchain
    pub nut15: Option<Nut15>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut4 {
    pub methods: Vec<(PaymentMethod, CurrencyUnit)>,
    pub disabled: bool,
}

impl Default for Nut4 {
    fn default() -> Self {
        Self {
            methods: vec![(PaymentMethod::Bolt11, CurrencyUnit::Sat)],
            disabled: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut5 {
    pub methods: Vec<(PaymentMethod, CurrencyUnit)>,
}

impl Default for Nut5 {
    fn default() -> Self {
        Self {
            methods: vec![(PaymentMethod::Bolt11, CurrencyUnit::Sat)],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut7 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut8 {
    pub supported: bool,
}

impl Default for Nut8 {
    fn default() -> Self {
        Self { supported: true }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut9 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut10 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut11 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut12 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut14 {
    pub supported: bool,
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct PaymentMethodConfig {
    pub payment_method: PaymentMethod,
    pub unit: CurrencyUnit,
    pub min_amount: u64,
    pub max_amount: u64,
}

impl Default for Nut14 {
    fn default() -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfig {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: 1_000,
                max_amount: 1_000_000,
            }],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut15 {
    pub supported: bool,
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfig>,
}

impl Default for Nut15 {
    fn default() -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfig {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: 1_000,
                max_amount: 1_000_000,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        dhke::public_key_from_hex,
        fixture::read_fixture,
        primitives::{
            KeyResponse, MintInfoResponse, MintLegacyInfoResponse, Nuts, Parameter,
            PostSwapResponse,
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
            nuts: Nuts::default(),
            motd: Some("Message to display to users.".to_string()),
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        println!("{}", out);
        assert!(!out.is_empty());
        // FIXME add asserts

        Ok(())
    }

    #[test]
    fn test_deserialize_nustash_mint_info() -> anyhow::Result<()> {
        let mint_info = read_fixture("nutshell_mint_info.json")?;
        let info = serde_json::from_str::<MintInfoResponse>(&mint_info);
        assert!(info.is_ok());
        let info = info?;
        assert_eq!("Nutshell/0.15.0", info.version.unwrap());
        Ok(())
    }
}
