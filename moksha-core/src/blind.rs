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

use crate::{
    amount::{generate_random_string, Amount},
    dhke::Dhke,
    error::MokshaCoreError,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedSignature {
    pub amount: u64,
    #[serde(rename = "C_")]
    #[schema(value_type=String)]
    pub c_: PublicKey,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    #[schema(value_type=String)]
    pub b_: PublicKey,
    pub id: String,
}

impl BlindedMessage {
    pub fn blank(
        fee_reserve: Amount,
        keyset_id: String,
    ) -> Result<Vec<(Self, SecretKey, String)>, MokshaCoreError> {
        if fee_reserve.0 == 0 {
            return Ok(vec![]);
        }

        let fee_reserve_float = fee_reserve.0 as f64;
        let count = (fee_reserve_float.log2().ceil() as u64).max(1);
        let dhke = Dhke::new();

        let blinded_messages = (0..count)
            .map(|_| {
                let secret = generate_random_string();
                let (b_, alice_secret_key) = dhke.step1_alice(secret.clone(), None).unwrap(); // FIXME
                (
                    Self {
                        amount: 1,
                        b_,
                        id: keyset_id.clone(),
                    },
                    alice_secret_key,
                    secret,
                )
            })
            .collect::<Vec<(Self, SecretKey, String)>>();

        Ok(blinded_messages)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1000_sats() {
        let result = BlindedMessage::blank(1000.into(), "00ffd48b8f5ecf80".to_owned());
        println!("{:?}", result);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.len() == 10);
        assert!(result.first().unwrap().0.amount == 1);
    }

    #[test]
    fn test_zero_sats() {
        let result = BlindedMessage::blank(0.into(), "00ffd48b8f5ecf80".to_owned());
        println!("{:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_serialize() -> anyhow::Result<()> {
        let result = BlindedMessage::blank(4000.into(), "00ffd48b8f5ecf80".to_owned())?;
        for (blinded_message, _, _) in result {
            let out = serde_json::to_string(&blinded_message)?;
            assert!(!out.is_empty());
        }
        Ok(())
    }
}
