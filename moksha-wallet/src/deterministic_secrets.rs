use crate::error::MokshaWalletError;
use std::str::FromStr;

use bip32::{Seed, XPrv};
use bip39::Mnemonic;
use rand::Rng;

pub enum DerivationType {
    Blinding = 0,
    Secret = 1,
}

pub fn derive_private_key(
    seed_words: &str,
    keyset_id: u32,
    counter: u32,
    secret_or_blinding: DerivationType,
) -> Result<String, MokshaWalletError> {
    let secret_or_blinding = secret_or_blinding as u32;
    let derivation_path = format!(
        "m/129372'/0'/{keyset_id}'/{counter}'/{secret_or_blinding}",
        keyset_id = keyset_id,
        counter = counter,
        secret_or_blinding = secret_or_blinding
    );

    let mnemonic = Mnemonic::from_str(seed_words)?;
    let seed = Seed::new(mnemonic.to_seed(""));
    let derivation_path = bip32::DerivationPath::from_str(&derivation_path).unwrap();
    let signing_key = XPrv::derive_from_path(seed, &derivation_path).unwrap();

    let priv_key = signing_key.private_key().to_bytes();
    let out = hex::encode(priv_key);

    Ok(out)
}

pub fn generate_mnemonic() -> Result<Mnemonic, bip39::Error> {
    let mut rng = rand::thread_rng();
    let entropy: [u8; 16] = rng.gen(); // 16 bytes for 12 words mnemonic
    let mnemonic = Mnemonic::from_entropy(&entropy)?;
    Ok(mnemonic)
}

#[cfg(test)]
mod tests {

    use super::{derive_private_key, DerivationType};

    #[test]
    fn test_() -> anyhow::Result<()> {
        let mn = super::generate_mnemonic()?;
        let words = mn.word_iter().collect::<Vec<_>>();

        println!("{:?}", words);

        let phrase =
            "half depart obvious quality work element tank gorilla view sugar picture humble";

        let secrets = [
            "485875df74771877439ac06339e284c3acfcd9be7abf3bc20b516faeadfe77ae",
            "8f2b39e8e594a4056eb1e6dbb4b0c38ef13b1b2c751f64f810ec04ee35b77270",
            "bc628c79accd2364fd31511216a0fab62afd4a18ff77a20deded7b858c9860c8",
            "59284fd1650ea9fa17db2b3acf59ecd0f2d52ec3261dd4152785813ff27a33bf",
            "576c23393a8b31cc8da6688d9c9a96394ec74b40fdaf1f693a6bb84284334ea0",
        ];

        for (i, secret) in secrets.iter().enumerate() {
            let key = derive_private_key(phrase, 864559728, i as u32, DerivationType::Blinding)?;
            assert_eq!(secret.to_owned(), key);
        }

        let blinding_factors = [
            "ad00d431add9c673e843d4c2bf9a778a5f402b985b8da2d5550bf39cda41d679",
            "967d5232515e10b81ff226ecf5a9e2e2aff92d66ebc3edf0987eb56357fd6248",
            "b20f47bb6ae083659f3aa986bfa0435c55c6d93f687d51a01f26862d9b9a4899",
            "fb5fca398eb0b1deb955a2988b5ac77d32956155f1c002a373535211a2dfdc29",
            "5f09bfbfe27c439a597719321e061e2e40aad4a36768bb2bcc3de547c9644bf9",
        ];

        for (i, factor) in blinding_factors.iter().enumerate() {
            let key = derive_private_key(phrase, 864559728, i as u32, DerivationType::Secret)?;
            assert_eq!(factor.to_owned(), key);
        }
        Ok(())
    }
}
