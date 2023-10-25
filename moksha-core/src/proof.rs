use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::error::MokshaCoreError;

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

#[cfg(test)]

mod tests {
    use serde_json::json;

    use crate::{
        fixture::read_fixture,
        proof::{Proof, Proofs},
        token::TokenV3,
    };

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
}
