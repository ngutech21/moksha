use bdk::bitcoin::bip32::{DerivationPath, ExtendedPrivKey};
use bdk::bitcoin::secp256k1::SecretKey;
use bdk::bitcoin::Network;
use bdk::keys::GeneratableKey;
use bdk::{
    keys::{
        bip39::{Language, Mnemonic, WordCount},
        GeneratedKey,
    },
    miniscript::Tap,
};

use std::str::FromStr;

use crate::error::MokshaWalletError;

pub fn generate_master_key() -> Result<ExtendedPrivKey, MokshaWalletError> {
    let mnemonic: GeneratedKey<_, Tap> =
        Mnemonic::generate((WordCount::Words12, Language::English))
            .expect("Cannot generate mnemonic");
    Ok(ExtendedPrivKey::new_master(
        Network::Bitcoin,
        &mnemonic.to_seed_normalized(""),
    )?)
}

pub enum DerivationType {
    Secret = 0,
    Blinding = 1,
}

pub fn derive_private_key(
    master: ExtendedPrivKey,
    keyset_id: u32,
    counter: u32,
    secret_or_blinding: DerivationType,
) -> Result<SecretKey, MokshaWalletError> {
    let derivation_path = format!(
        "m/129372'/{}'/{}'/{}",
        keyset_id, counter, secret_or_blinding as u32
    );
    let derivation_path = DerivationPath::from_str(&derivation_path)?;

    Ok(master
        .derive_priv(&bdk::bitcoin::secp256k1::Secp256k1::new(), &derivation_path)?
        .private_key)
}
