//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display, str::FromStr};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesRequest {
    pub pr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesResponse {
    /// fee in satoshis
    pub fee: u64,
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
    MSat,
    Usd,
}

impl From<String> for CurrencyUnit {
    fn from(unit: String) -> Self {
        match unit.to_lowercase().as_str() {
            "sat" => Self::Sat,
            "msat" => Self::Sat,
            "usd" => Self::Usd,
            _ => panic!("Unknown currency unit: {}", unit),
        }
    }
}

impl Display for CurrencyUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sat => write!(f, "sat"),
            Self::MSat => write!(f, "msat"),
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
    pub outputs: Option<Vec<BlindedMessage>>,
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
    pub contact: Option<Vec<ContactInfoResponse>>,
    pub motd: Option<String>,
    pub nuts: Nuts,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct ContactInfoResponse {
    pub method: String,
    pub info: String,
}

impl ContactInfoResponse {
    pub fn email(info: String) -> Self {
        Self {
            method: "email".to_string(),
            info,
        }
    }

    pub fn twitter(info: String) -> Self {
        Self {
            method: "twitter".to_string(),
            info,
        }
    }

    pub fn nostr(info: String) -> Self {
        Self {
            method: "twitter".to_string(),
            info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcOnchainMintQuote {
    pub quote_id: Uuid,
    pub address: String,
    pub unit: CurrencyUnit,
    pub amount: u64,
    pub expiry: u64,
    pub state: MintBtcOnchainState,
}

impl From<BtcOnchainMintQuote> for PostMintQuoteBtcOnchainResponse {
    fn from(quote: BtcOnchainMintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            address: quote.address,
            state: quote.state,
            expiry: quote.expiry,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcOnchainMeltQuote {
    pub quote_id: Uuid,
    pub amount: u64,
    pub address: String,
    pub fee_total: u64,
    pub fee_sat_per_vbyte: u32,
    pub expiry: u64,
    pub state: MeltBtcOnchainState,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainRequest {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainResponse {
    pub quote: String,
    pub address: String,
    pub state: MintBtcOnchainState,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum MintBtcOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    Pending,

    Paid,

    Issued,
}

impl Display for MintBtcOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MintBtcOnchainState::Unpaid => write!(f, "UNPAID"),
            MintBtcOnchainState::Pending => write!(f, "PENDING"),
            MintBtcOnchainState::Paid => write!(f, "PAID"),
            MintBtcOnchainState::Issued => write!(f, "ISSUED"),
        }
    }
}

impl FromStr for MintBtcOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MintBtcOnchainState::Unpaid),
            "PENDING" => Ok(MintBtcOnchainState::Pending),
            "PAID" => Ok(MintBtcOnchainState::Paid),
            "ISSUED" => Ok(MintBtcOnchainState::Issued),
            _ => Err(()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBtcOnchainRequest {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBtcOnchainResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBtcOnchainRequest {
    pub amount: u64,
    /// onchain address
    pub address: String,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBtcOnchainResponse {
    pub quote: String,
    pub description: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub state: MeltBtcOnchainState,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBtcOnchainRequest {
    pub quote: String,
    pub inputs: Proofs,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBtcOnchainResponse {
    pub state: MeltBtcOnchainState,
    pub txid: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum MeltBtcOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    /// the mint received the payment from the wallet, but did not broadcast the transaction yet
    Pending,

    /// the mint broadcasted the btc onchain transaction
    Paid,
}

impl Display for MeltBtcOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeltBtcOnchainState::Unpaid => write!(f, "UNPAID"),
            MeltBtcOnchainState::Pending => write!(f, "PENDING"),
            MeltBtcOnchainState::Paid => write!(f, "PAID"),
        }
    }
}

impl FromStr for MeltBtcOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MeltBtcOnchainState::Unpaid),
            "PENDING" => Ok(MeltBtcOnchainState::Pending),
            "PAID" => Ok(MeltBtcOnchainState::Paid),
            _ => Err(()),
        }
    }
}

impl From<BtcOnchainMeltQuote> for PostMeltQuoteBtcOnchainResponse {
    fn from(quote: BtcOnchainMeltQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            amount: quote.amount,
            fee: quote.fee_total,
            expiry: quote.expiry,
            state: quote.state,
            description: quote.description,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nuts {
    /// Minting tokens
    #[serde(rename = "4")]
    pub nut4: Nut4,

    /// Melting tokens
    #[serde(rename = "5")]
    pub nut5: Nut5,

    /// Token state check
    #[serde(rename = "7", skip_serializing_if = "Option::is_none")]
    pub nut7: Option<Nut7>,

    /// Overpaid Lightning fees
    #[serde(rename = "8", skip_serializing_if = "Option::is_none")]
    pub nut8: Option<Nut8>,

    /// Deterministic backup and restore
    #[serde(rename = "9", skip_serializing_if = "Option::is_none")]
    pub nut9: Option<Nut9>,

    /// Spending conditions
    #[serde(rename = "10", skip_serializing_if = "Option::is_none")]
    pub nut10: Option<Nut10>,

    /// Pay-To-Pubkey (P2PK)
    #[serde(rename = "11", skip_serializing_if = "Option::is_none")]
    pub nut11: Option<Nut11>,

    #[serde(rename = "12", skip_serializing_if = "Option::is_none")]
    /// DLEQ proofs
    pub nut12: Option<Nut12>,

    #[serde(rename = "13", skip_serializing_if = "Option::is_none")]
    /// deterministic secrets
    pub nut13: Option<Nut13>,

    #[serde(rename = "14", skip_serializing_if = "Option::is_none")]
    /// Hashed Timelock Contracts
    pub nut14: Option<Nut14>,

    #[serde(rename = "15", skip_serializing_if = "Option::is_none")]
    /// Partial multi-path payments
    pub nut15: Option<Nut15>,

    #[serde(rename = "16", skip_serializing_if = "Option::is_none")]
    /// Partial multi-path payments
    pub nut16: Option<Nut16>,

    #[serde(rename = "17", skip_serializing_if = "Option::is_none")]
    /// WebSockets
    pub nut17: Option<Nut17>,

    #[serde(rename = "18", skip_serializing_if = "Option::is_none")]
    /// minting tokens btc onchain
    pub nut18: Option<Nut18>,

    #[serde(rename = "19", skip_serializing_if = "Option::is_none")]
    /// melting tokens btc onchain
    pub nut19: Option<Nut19>,
}

impl Default for Nuts {
    fn default() -> Self {
        Self {
            nut4: Nut4::default(),
            nut5: Nut5::default(),
            nut7: Some(Nut7 { supported: false }),
            nut8: Some(Nut8 { supported: true }),
            nut9: Some(Nut9 { supported: false }),
            nut10: Some(Nut10 { supported: false }),
            nut11: Some(Nut11 { supported: false }),
            nut12: Some(Nut12 { supported: false }),
            nut13: Some(Nut13::default()),
            nut14: Some(Nut14::default()),
            nut15: Some(Nut15::default()),
            nut16: Some(Nut16::default()),
            nut17: Some(Nut17::default()),
            nut18: Some(Nut18::default()),
            nut19: Some(Nut19::default()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut4 {
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfig>,
    pub disabled: bool,
}

impl Default for Nut4 {
    fn default() -> Self {
        Self {
            payment_methods: vec![PaymentMethodConfig {
                payment_method: PaymentMethod::Bolt11,
                unit: CurrencyUnit::Sat,
                min_amount: Some(1),
                max_amount: Some(10_000_000),
            }],
            disabled: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut5 {
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfig>,
    pub disabled: bool,
}

impl Default for Nut5 {
    fn default() -> Self {
        Self {
            payment_methods: vec![PaymentMethodConfig {
                payment_method: PaymentMethod::Bolt11,
                unit: CurrencyUnit::Sat,
                min_amount: Some(1),
                max_amount: Some(10_000_000),
            }],
            disabled: false,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut13 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut14 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut15 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut16 {
    pub supported: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Nut17 {}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut18 {
    pub supported: bool,
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfigBtcOnchainMint>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct PaymentMethodConfig {
    #[serde(rename = "method")]
    pub payment_method: PaymentMethod,
    pub unit: CurrencyUnit,
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct PaymentMethodConfigBtcOnchainMint {
    #[serde(rename = "method")]
    pub payment_method: PaymentMethod,
    pub unit: CurrencyUnit,
    pub min_amount: u64,
    pub max_amount: u64,
    pub min_confirmations: u8,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct PaymentMethodConfigBtcOnchainMelt {
    #[serde(rename = "method")]
    pub payment_method: PaymentMethod,
    pub unit: CurrencyUnit,
    pub min_amount: u64,
    pub max_amount: u64,
}

impl Default for Nut18 {
    fn default() -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfigBtcOnchainMint {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: 10_000,
                max_amount: 1_000_000,
                min_confirmations: 3,
            }],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct Nut19 {
    pub supported: bool,
    #[serde(rename = "methods")]
    pub payment_methods: Vec<PaymentMethodConfigBtcOnchainMelt>,
}

impl Default for Nut19 {
    fn default() -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfigBtcOnchainMelt {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: 10_000,
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
        primitives::{ContactInfoResponse, KeyResponse, MintInfoResponse, Nuts, PostSwapResponse},
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
    fn test_serialize_mint_info() -> anyhow::Result<()> {
        let mint_info = MintInfoResponse {
            name: Some("Bob's Cashu mint".to_string()),
            pubkey: public_key_from_hex(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
            ),
            version: Some("Nutshell/0.11.0".to_string()),
            description: Some("The short mint description".to_string()),
            description_long: Some("A description that can be a long piece of text.".to_string()),
            contact: Some(vec![
                ContactInfoResponse::email("contact@me.com".to_string()),
                ContactInfoResponse::twitter("@me".to_string()),
                ContactInfoResponse::nostr("npub...".to_string()),
            ]),
            nuts: Nuts::default(),
            motd: Some("Message to display to users.".to_string()),
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        assert!(!out.is_empty());
        assert!(out.contains("02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2"));
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

    #[test]
    fn test_deserialize_nustash_v_0_16_0_mint_info() -> anyhow::Result<()> {
        let mint_info = read_fixture("nutshell_mint_info_v0.16.0.json")?;
        let info = serde_json::from_str::<MintInfoResponse>(&mint_info);
        assert!(info.is_ok());
        let info = info?;
        assert_eq!("Nutshell/0.16.0", info.version.unwrap());
        Ok(())
    }

    #[test]
    fn test_deserialize_incomplete_mint_info() -> anyhow::Result<()> {
        let mint_info = read_fixture("incomplete_mint_info.json")?;
        let info = serde_json::from_str::<MintInfoResponse>(&mint_info);
        assert!(info.is_ok());
        let info = info?;
        assert_eq!("MyMint", info.version.unwrap());
        assert!(info.nuts.nut7.is_none());
        assert!(info.nuts.nut8.is_none());
        assert!(info.nuts.nut9.is_none());
        assert!(info.nuts.nut10.is_none());
        assert!(info.nuts.nut11.is_none());
        assert!(info.nuts.nut12.is_none());
        Ok(())
    }
}
