use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use base64::{engine::general_purpose, Engine as _};
use bitcoin_hashes::{sha256, Hash};

use itertools::Itertools;
use rand::RngCore;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

use crate::error::MokshaCoreError;

const MAX_ORDER: u64 = 64;

pub fn generate_hash() -> String {
    let mut rng = rand::thread_rng();
    let mut random = [0u8; 32];
    rng.fill_bytes(&mut random);
    sha256::Hash::hash(&random).to_string()
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

    pub fn current_keyset(
        &self,
        mint_keys: &HashMap<u64, PublicKey>,
    ) -> Result<String, MokshaCoreError> {
        let computed_id = derive_keyset_id(mint_keys);
        if self.keysets.contains(&computed_id) {
            Ok(computed_id)
        } else {
            Err(MokshaCoreError::InvalidKeysetid)
        }
    }
}

/// Derives a set of secret keys from a master key using a given derivation path.
///
/// # Arguments
///
/// * `master_key` - A string slice that holds the master key.
/// * `derivation_path` - A string slice that holds the derivation path.
///
/// # Returns
///
/// A HashMap containing the derived secret keys, where the key is a u64 value and the value is a SecretKey.
pub fn derive_keys(master_key: &str, derivation_path: &str) -> HashMap<u64, SecretKey> {
    let mut keys = HashMap::new();
    for i in 0..MAX_ORDER {
        let hash = sha256::Hash::hash(format!("{master_key}{derivation_path}{i}").as_bytes());
        let key = SecretKey::from_slice(hash.as_byte_array()).unwrap();
        keys.insert(2u64.pow(i as u32), key);
    }
    keys
}

/// Derives public keys from a given set of secret keys.
///
/// # Arguments
///
/// * `keys` - A HashMap containing the secret keys to derive public keys from.
///
/// # Returns
///
/// A HashMap containing the derived public keys.
pub fn derive_pubkeys(keys: &HashMap<u64, SecretKey>) -> HashMap<u64, PublicKey> {
    let secp = Secp256k1::new();
    keys.keys()
        .map(|amt| (*amt, keys[amt].public_key(&secp)))
        .collect()
}

/// Derives a keyset ID from a HashMap of public keys.
///
/// # Arguments
///
/// * `keys` - A HashMap of public keys.
///
/// # Returns
///
/// A string representing the derived keyset ID.
pub fn derive_keyset_id(keys: &HashMap<u64, PublicKey>) -> String {
    let pubkeys_concat = keys
        .iter()
        .sorted_by(|(amt_a, _), (amt_b, _)| amt_a.cmp(amt_b))
        .map(|(_, pubkey)| pubkey)
        .join("");
    let hashed_pubkeys = sha256::Hash::hash(pubkeys_concat.as_bytes()).to_byte_array();
    general_purpose::STANDARD.encode(hashed_pubkeys)[..12].to_string()
}

/// Derives a public key from a given seed.
///
/// # Arguments
///
/// * `seed` - A string slice representing the seed to derive the public key from.
///
/// # Returns
///
/// Returns a `Result` containing the derived `PublicKey` or a `MokshaCoreError` if an error occurs.
pub fn derive_pubkey(seed: &str) -> Result<PublicKey, MokshaCoreError> {
    let hash = sha256::Hash::hash(seed.as_bytes());
    let key = SecretKey::from_slice(hash.as_byte_array())?;
    let secp = Secp256k1::new();
    Ok(key.public_key(&secp))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::keyset::{derive_pubkey, generate_hash};

    fn public_key_from_hex(hex: &str) -> secp256k1::PublicKey {
        use hex::FromHex;
        let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
        secp256k1::PublicKey::from_slice(&input_vec).expect("Invalid Public Key")
    }

    #[test]
    fn test_generate_hash() {
        let hash = generate_hash();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_derive_pubkey() -> anyhow::Result<()> {
        let result = derive_pubkey("supersecretprivatekey")?;
        assert_eq!(
            "03a2118b421e6b47f0656b97bb7eeea43c41096adbc0d0e511ff70de7d94dbd990",
            result.to_string()
        );
        Ok(())
    }

    #[test]
    fn test_derive_keys_master() -> anyhow::Result<()> {
        let keys = super::derive_keys("master", "0/0/0/0");
        assert!(keys.len() == 64);

        let pub_keys = super::derive_pubkeys(&keys);
        let id = super::derive_keyset_id(&pub_keys);
        assert_eq!("JHV8eUnoAln/", id);
        assert!(id.len() == 12);
        Ok(())
    }

    // uses values from cashu test_mint.py
    #[test]
    fn test_derive_keys_cashu_py() -> anyhow::Result<()> {
        let keys = super::derive_keys("TEST_PRIVATE_KEY", "0/0/0/0");
        assert!(keys.len() == 64);

        let pub_keys = super::derive_pubkeys(&keys);
        let id = super::derive_keyset_id(&pub_keys);
        assert_eq!("1cCNIAZ2X/w1", id);
        assert!(id.len() == 12);
        Ok(())
    }

    #[test]
    fn test_derive_keyset_id() -> anyhow::Result<()> {
        let mut pubs = HashMap::new();
        pubs.insert(
            1,
            public_key_from_hex(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
            ),
        );

        pubs.insert(
            2,
            public_key_from_hex(
                "020000000000000000000000000000000000000000000000000000000000000001",
            ),
        );

        let keyset_id = super::derive_keyset_id(&pubs);

        assert!(keyset_id.len() == 12);
        assert_eq!(keyset_id, "cNbjM0O6V/Kl");
        Ok(())
    }
}
