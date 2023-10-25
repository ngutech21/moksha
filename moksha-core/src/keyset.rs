use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    crypto::{self, derive_keys, derive_keyset_id, derive_pubkey, derive_pubkeys},
    error::MokshaCoreError,
};

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
        let computed_id = crypto::derive_keyset_id(mint_keys);
        if self.keysets.contains(&computed_id) {
            Ok(computed_id)
        } else {
            Err(MokshaCoreError::InvalidKeysetid)
        }
    }
}
