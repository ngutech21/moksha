use base64::{engine::general_purpose, Engine as _};
use bitcoin_hashes::{sha256, Hash};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::collections::HashMap;

const MAX_ORDER: u64 = 32;

pub fn derive_keys(master_key: &str, derivation_path: &str) -> HashMap<u64, SecretKey> {
    let mut keys = HashMap::new();
    for i in 0..MAX_ORDER {
        let hash = sha256::Hash::hash(format!("{}{}{}", master_key, derivation_path, i).as_bytes());
        let key = SecretKey::from_slice(hash.as_byte_array()).unwrap();
        keys.insert(2u64.pow(i as u32), key);
    }
    keys
}

pub fn derive_pubkey(master_key: &str) -> PublicKey {
    let secp = Secp256k1::new();
    let binding = sha256::Hash::hash(master_key.as_bytes());
    let hash = binding.as_byte_array();
    let private_key = SecretKey::from_slice(hash).unwrap();
    private_key.public_key(&secp)
}

pub fn derive_pubkeys(keys: HashMap<i32, SecretKey>) -> HashMap<i32, PublicKey> {
    let secp = Secp256k1::new();
    let mut pubkeys: HashMap<i32, PublicKey> = HashMap::new();
    for amt in keys.keys() {
        pubkeys.insert(*amt, keys[amt].public_key(&secp));
    }
    pubkeys
}

pub fn derive_keyset_id(keys: HashMap<i32, PublicKey>) -> String {
    let mut sorted_keys = keys.keys().collect::<Vec<&i32>>();
    sorted_keys.sort();
    let pubkeys_concat = sorted_keys
        .iter()
        .map(|p| p.to_string())
        .collect::<String>();
    let hashed_pubkeys = sha256::Hash::hash(pubkeys_concat.as_bytes()).to_byte_array();
    general_purpose::URL_SAFE.encode(hashed_pubkeys)[..12].to_string()
}
