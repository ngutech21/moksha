use secp256k1::PublicKey;
use serde_derive::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MintInfoSettings {
    pub name: Option<String>,
    #[serde(default)]
    pub version: bool,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
}

#[skip_serializing_none]
#[derive(serde::Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct MintInfoResponse {
    pub name: Option<String>,
    pub pubkey: PublicKey,
    pub version: Option<String>,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub nuts: Vec<String>,
    pub motd: Option<String>,
    pub parameter: Parameter,
}

#[derive(serde::Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub peg_out_only: bool,
}

#[derive(serde::Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Contact {
    Email(String),
    Twitter(String),
    Nostr(String),
}

#[cfg(test)]
mod tests {

    use super::*;

    fn public_key_from_hex(hex: &str) -> secp256k1::PublicKey {
        use hex::FromHex;
        let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
        secp256k1::PublicKey::from_slice(&input_vec).expect("Invalid Public Key")
    }

    #[test]
    fn test_deserialize_mint_info() -> anyhow::Result<()> {
        let mint_info = MintInfoResponse {
            name: Some("Bob's Cashu mint".to_string()),
            pubkey: public_key_from_hex(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
            ),
            version: Some("Nutshell/0.11.0".to_string()),
            description: Some("The short mint description".to_string()),
            description_long: Some("A description that can be a long piece of text.".to_string()),
            contact: Some(vec![
                vec!["email".to_string(), "contact@me.com".to_string()],
                vec!["twitter".to_string(), "@me".to_string()],
                vec!["nostr".to_string(), "npub...".to_string()],
            ]),
            nuts: vec![
                "NUT-07".to_string(),
                "NUT-08".to_string(),
                "NUT-08".to_string(),
            ],
            motd: Some("Message to display to users.".to_string()),
            parameter: Parameter {
                peg_out_only: false,
            },
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        println!("{}", out);
        assert!(!out.is_empty());

        Ok(())
    }
}
