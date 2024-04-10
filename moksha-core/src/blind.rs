//! This module defines the `BlindedMessage` and `BlindedSignature` structs, which are used for representing blinded messages and signatures in Cashu as described in [Nut-00](https://github.com/cashubtc/nuts/blob/main/00.md)
//!
//! The `BlindedMessage` struct represents a blinded message, with an `amount` field for the amount in satoshis and a `b_` field for the public key of the blinding factor.
//!
//! The `BlindedSignature` struct represents a blinded signature, with an `amount` field for the amount in satoshis, a `c_` field for the public key of the blinding factor, and an optional `id` field for the ID of the signature.
//!
//! Both the `BlindedMessage` and `BlindedSignature` structs are serializable and deserializable using serde.
//!
//! The `TotalAmount` trait is also defined in this module, which provides a `total_amount` method for calculating the total amount of a vector of `BlindedMessage` or `BlindedSignature` structs. The trait is implemented for both `Vec<BlindedMessage>` and `Vec<BlindedSignature>`.

use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::MokshaCoreError;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedSignature {
    pub amount: u64,
    #[serde(rename = "C_")]
    #[schema(value_type=String)]
    pub c_: PublicKey,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    #[schema(value_type=String)]
    pub b_: PublicKey,
    // FIXME use KeysetId
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct BlindingFactor(SecretKey);

impl From<SecretKey> for BlindingFactor {
    fn from(sk: SecretKey) -> Self {
        BlindingFactor(sk)
    }
}

impl TryFrom<&str> for BlindingFactor {
    type Error = MokshaCoreError;

    fn try_from(hex: &str) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        Ok(secp256k1::SecretKey::from_str(hex)?.into())
    }
}

impl BlindingFactor {
    pub fn as_hex(&self) -> String {
        hex::encode(&self.0[..])
    }

    pub fn to_secret_key(&self) -> SecretKey {
        self.0
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

impl TotalAmount for Vec<BlindedMessage> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}
