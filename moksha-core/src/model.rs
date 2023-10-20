use base64::{engine::general_purpose, Engine as _};
use rand::distributions::Alphanumeric;
use rand::Rng;
use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use url::Url;

use crate::{
    crypto::{self, derive_keys, derive_keyset_id, derive_pubkey, derive_pubkeys},
    error::MokshaCoreError,
};

const TOKEN_PREFIX_V3: &str = "cashuA";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    pub b_: PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSignature {
    pub amount: u64,
    #[serde(rename = "C_")]
    pub c_: PublicKey,
    pub id: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Proof {
    pub amount: u64,
    pub secret: String,
    #[serde(rename = "C")]
    pub c: PublicKey,
    pub id: String, // FIXME use keysetID as specific type / consider making this non optional and brake backwards compatibility
    pub script: Option<P2SHScript>,
}

impl Proof {
    pub fn new(amount: u64, secret: String, c: PublicKey, id: String) -> Self {
        Self {
            amount,
            secret,
            c,
            id,
            script: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2SHScript;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Token {
    #[serde(serialize_with = "serialize_url", deserialize_with = "deserialize_url")]
    pub mint: Option<Url>,
    pub proofs: Proofs,
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str: Option<String> = Option::deserialize(deserializer)?;
    match url_str {
        Some(s) => Url::parse(&s).map_err(serde::de::Error::custom).map(Some),
        None => Ok(None),
    }
}

fn serialize_url<S>(url: &Option<Url>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match url {
        Some(url) => {
            let mut url_str = url.as_str().to_owned();
            if url_str.ends_with('/') {
                url_str.pop();
            }
            serializer.serialize_str(&url_str)
        }
        None => serializer.serialize_none(),
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenV3 {
    #[serde(rename = "token")]
    pub tokens: Vec<Token>,
    pub memo: Option<String>,
}

impl TokenV3 {
    pub fn new(token: Token) -> Self {
        Self {
            tokens: vec![token],
            memo: None,
        }
    }

    pub fn total_amount(&self) -> u64 {
        self.tokens
            .iter()
            .map(|token| {
                token
                    .proofs
                    .proofs()
                    .iter()
                    .map(|proof| proof.amount)
                    .sum::<u64>()
            })
            .sum()
    }

    pub fn proofs(&self) -> Proofs {
        Proofs::new(
            self.tokens
                .iter()
                .flat_map(|token| token.proofs.proofs())
                .collect(),
        )
    }

    pub fn serialize(&self) -> Result<String, MokshaCoreError> {
        let json = serde_json::to_string(&self)?;
        Ok(format!(
            "{}{}",
            TOKEN_PREFIX_V3,
            general_purpose::URL_SAFE.encode(json.as_bytes())
        ))
    }

    pub fn deserialize(data: String) -> Result<TokenV3, MokshaCoreError> {
        let json = general_purpose::URL_SAFE.decode(
            data.strip_prefix(TOKEN_PREFIX_V3)
                .ok_or(MokshaCoreError::InvalidTokenPrefix)?
                .as_bytes(),
        )?;
        Ok(serde_json::from_slice::<TokenV3>(&json)?)
    }

    pub fn mint(&self) -> Option<Url> {
        self.tokens
            .first()
            .and_then(|token| token.mint.as_ref())
            .map(|url| url.to_owned())
    }
}

impl TryFrom<TokenV3> for String {
    type Error = MokshaCoreError;

    fn try_from(token: TokenV3) -> Result<Self, Self::Error> {
        token.serialize()
    }
}

impl TryFrom<String> for TokenV3 {
    type Error = MokshaCoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::deserialize(value)
    }
}

impl From<(Url, Proofs)> for TokenV3 {
    fn from(from: (Url, Proofs)) -> Self {
        Self {
            tokens: vec![Token {
                mint: Some(from.0),
                proofs: from.1,
            }],
            memo: None,
        }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Proofs(pub(super) Vec<Proof>);

impl Proofs {
    pub fn new(proofs: Vec<Proof>) -> Self {
        Self(proofs)
    }

    pub fn with_proof(proof: Proof) -> Self {
        Self(vec![proof])
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn total_amount(&self) -> u64 {
        self.0.iter().map(|proof| proof.amount).sum()
    }

    pub fn proofs(&self) -> Vec<Proof> {
        self.0.clone()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn proofs_for_amount(&self, amount: u64) -> Result<Proofs, MokshaCoreError> {
        let mut all_proofs = self.0.clone();
        if amount > self.total_amount() {
            return Err(MokshaCoreError::NotEnoughTokens);
        }

        all_proofs.sort_by(|a, b| a.amount.cmp(&b.amount));

        let mut selected_proofs = vec![];
        let mut selected_amount = 0;

        while selected_amount < amount {
            if all_proofs.is_empty() {
                break;
            }

            let proof = all_proofs.pop().expect("proofs is empty");
            selected_amount += proof.amount;
            selected_proofs.push(proof);
        }

        Ok(selected_proofs.into())
    }
}

impl From<Vec<Proof>> for Proofs {
    fn from(from: Vec<Proof>) -> Self {
        Self(from)
    }
}

impl From<Proof> for Proofs {
    fn from(from: Proof) -> Self {
        Self(vec![from])
    }
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

    pub fn get_current_keyset(
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub pr: String,
    pub hash: String, // TODO use sha256::Hash
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostMintResponse {
    pub promises: Vec<BlindedSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMintRequest {
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesRequest {
    pub pr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckFeesResponse {
    pub fee: u64, // fee in satoshis
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostMeltRequest {
    pub proofs: Proofs,
    pub pr: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostMeltResponse {
    pub paid: bool,
    pub preimage: String,
    pub change: Vec<BlindedSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostSplitRequest {
    pub proofs: Proofs,
    pub outputs: Vec<BlindedMessage>,
    pub amount: Option<u64>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostSplitResponse {
    pub fst: Option<Vec<BlindedSignature>>,
    pub snd: Option<Vec<BlindedSignature>>,
    pub promises: Option<Vec<BlindedSignature>>,
}

impl PostSplitResponse {
    pub fn with_promises(promises: Vec<BlindedSignature>) -> Self {
        Self {
            promises: Some(promises),
            ..Default::default()
        }
    }

    pub fn with_fst_and_snd(fst: Vec<BlindedSignature>, snd: Vec<BlindedSignature>) -> Self {
        Self {
            fst: Some(fst),
            snd: Some(snd),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct Amount(pub u64);

impl Amount {
    pub fn split(&self) -> SplitAmount {
        split_amount(self.0).into()
    }
}

#[derive(Clone)]
pub struct SplitAmount(pub Vec<u64>);

impl From<Vec<u64>> for SplitAmount {
    fn from(from: Vec<u64>) -> Self {
        Self(from)
    }
}

impl SplitAmount {
    pub fn create_secrets(&self) -> Vec<String> {
        (0..self.0.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }
}

impl From<u64> for Amount {
    fn from(amount: u64) -> Self {
        Self(amount)
    }
}

/// split a decimal amount into a vector of powers of 2
pub fn split_amount(amount: u64) -> Vec<u64> {
    format!("{amount:b}")
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '1' {
                return Some(2_u64.pow(i as u32));
            }
            None
        })
        .collect::<Vec<u64>>()
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[derive(serde::Deserialize, Debug)]
pub struct CashuErrorResponse {
    pub code: u64,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use crate::{
        dhke,
        fixture::read_fixture,
        model::{PostSplitResponse, Proof, Proofs, SplitAmount, Token, TokenV3},
    };
    use serde_json::{json, Value};
    use url::Url;

    #[test]
    fn test_serialize_empty_split_response() -> anyhow::Result<()> {
        let response = PostSplitResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{}");
        Ok(())
    }

    #[test]
    fn test_proofs_for_amount_empty() -> anyhow::Result<()> {
        let proofs = Proofs::empty();

        let result = proofs.proofs_for_amount(10);

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("Not enough tokens"));
        Ok(())
    }

    #[test]
    fn test_proofs_for_amount_valid() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let token: TokenV3 = fixture.try_into()?;

        let result = token.proofs().proofs_for_amount(10)?;
        assert_eq!(32, result.total_amount());
        assert_eq!(1, result.len());
        Ok(())
    }

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let bits = super::split_amount(13);
        assert_eq!(bits, vec![1, 4, 8]);

        let bits = super::split_amount(63);
        assert_eq!(bits, vec![1, 2, 4, 8, 16, 32]);

        let bits = super::split_amount(64);
        assert_eq!(bits, vec![64]);
        Ok(())
    }

    #[test]
    fn test_create_secrets() {
        let amounts = vec![1, 2, 3, 4, 5, 6, 7];
        let secrets = SplitAmount::from(amounts.clone()).create_secrets();
        assert!(secrets.len() == amounts.len());
        assert_eq!(secrets.get(0).unwrap().len(), 24);
    }

    #[test]
    fn test_proof() -> anyhow::Result<()> {
        let js = json!(
            {
              "id": "DSAl9nvvyfva",
              "amount": 2,
              "secret": "EhpennC9qB3iFlW8FZ_pZw",
              "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
            }
        );

        let proof = serde_json::from_value::<Proof>(js)?;
        assert_eq!(proof.amount, 2);
        assert_eq!(proof.id, "DSAl9nvvyfva".to_string());
        assert_eq!(proof.secret, "EhpennC9qB3iFlW8FZ_pZw".to_string());
        assert_eq!(
            proof.c.to_string(),
            "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4".to_string()
        );
        Ok(())
    }

    #[test]
    fn test_token() -> anyhow::Result<()> {
        let js = json!(
            {
              "mint": "https://8333.space:3338",
              "proofs": [
                {
                  "id": "DSAl9nvvyfva",
                  "amount": 2,
                  "secret": "EhpennC9qB3iFlW8FZ_pZw",
                  "C": "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4"
                },
                {
                  "id": "DSAl9nvvyfva",
                  "amount": 8,
                  "secret": "TmS6Cv0YT5PU_5ATVKnukw",
                  "C": "02ac910bef28cbe5d7325415d5c263026f15f9b967a079ca9779ab6e5c2db133a7"
                }
              ]
        });

        let token = serde_json::from_value::<super::Token>(js)?;
        assert_eq!(token.mint, Some(Url::parse("https://8333.space:3338")?));
        assert_eq!(token.proofs.len(), 2);
        Ok(())
    }

    #[test]
    fn test_tokens_serialize() -> anyhow::Result<()> {
        use base64::{engine::general_purpose, Engine as _};
        let token = Token {
            mint: Some(Url::parse("https://8333.space:3338/")?),
            proofs: Proof {
                amount: 21,
                secret: "secret".to_string(),
                c: dhke::public_key_from_hex(
                    "02c020067db727d586bc3183aecf97fcb800c3f4cc4759f69c626c9db5d8f5b5d4",
                ),
                id: "someid".to_string(),
                script: None,
            }
            .into(),
        };
        let tokens = super::TokenV3 {
            tokens: vec![token],
            memo: Some("my memo".to_string()),
        };

        let serialized: String = tokens.try_into()?;
        assert!(serialized.starts_with("cashuA"));

        // check if mint is serialized without trailing slash
        let json = general_purpose::URL_SAFE.decode(serialized.strip_prefix("cashuA").unwrap())?;
        let deser = String::from_utf8(json)?;
        let json: Value = serde_json::from_str(&deser)?;
        let mint_value = json["token"][0]["mint"].as_str();
        assert_eq!(mint_value, Some("https://8333.space:3338"));
        Ok(())
    }

    #[test]
    fn test_tokens_deserialize() -> anyhow::Result<()> {
        let input = "cashuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJpZCI6IkRTQWw5bnZ2eWZ2YSIsImFtb3VudCI6Miwic2VjcmV0IjoiRWhwZW5uQzlxQjNpRmxXOEZaX3BadyIsIkMiOiIwMmMwMjAwNjdkYjcyN2Q1ODZiYzMxODNhZWNmOTdmY2I4MDBjM2Y0Y2M0NzU5ZjY5YzYyNmM5ZGI1ZDhmNWI1ZDQifSx7ImlkIjoiRFNBbDludnZ5ZnZhIiwiYW1vdW50Ijo4LCJzZWNyZXQiOiJUbVM2Q3YwWVQ1UFVfNUFUVktudWt3IiwiQyI6IjAyYWM5MTBiZWYyOGNiZTVkNzMyNTQxNWQ1YzI2MzAyNmYxNWY5Yjk2N2EwNzljYTk3NzlhYjZlNWMyZGIxMzNhNyJ9XX1dLCJtZW1vIjoiVGhhbmt5b3UuIn0=";
        let tokens = TokenV3::deserialize(input.to_string())?;
        assert_eq!(tokens.memo, Some("Thankyou.".to_string()),);
        assert_eq!(tokens.tokens.len(), 1);
        Ok(())
    }
}
