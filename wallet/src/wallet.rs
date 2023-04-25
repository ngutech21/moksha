use std::collections::HashMap;

use cashurs_core::model::{Keysets, Tokens};
use secp256k1::PublicKey;

use crate::client::Client;

pub struct Wallet {
    client: Client,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
}

impl Wallet {
    pub fn new(client: Client, mint_keys: HashMap<u64, PublicKey>, keysets: Keysets) -> Self {
        Self {
            client,
            mint_keys,
            keysets,
        }
    }

    pub fn melt_token(&self, Tokens: Tokens) {}
}
