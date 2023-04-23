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

pub fn derive_pubkeys(keys: &HashMap<u64, SecretKey>) -> HashMap<u64, PublicKey> {
    let secp = Secp256k1::new();
    let mut pubkeys: HashMap<u64, PublicKey> = HashMap::new();
    for amt in keys.keys() {
        pubkeys.insert(*amt, keys[amt].public_key(&secp));
    }
    pubkeys
}

pub fn derive_keyset_id(keys: &HashMap<u64, PublicKey>) -> String {
    let mut sorted_keys = keys.keys().collect::<Vec<&u64>>();
    sorted_keys.sort();
    let pubkeys_concat = sorted_keys
        .iter()
        .map(|p| p.to_string())
        .collect::<String>();
    let hashed_pubkeys = sha256::Hash::hash(pubkeys_concat.as_bytes()).to_byte_array();
    general_purpose::URL_SAFE.encode(hashed_pubkeys)[..12].to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    fn public_key_from_hex(hex: &str) -> secp256k1::PublicKey {
        use hex::FromHex;
        let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
        secp256k1::PublicKey::from_slice(&input_vec).expect("Invalid Public Key")
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
        assert_eq!(keyset_id, "a1HUMd9dfxQc");
        Ok(())
    }
}
