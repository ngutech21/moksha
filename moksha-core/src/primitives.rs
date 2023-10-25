use serde::{Deserialize, Serialize};

use crate::{
    blind::{BlindedMessage, BlindedSignature},
    proof::Proofs,
};

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
    /// fee in satoshis
    pub fee: u64,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PostSplitResponse {
    pub promises: Vec<BlindedSignature>,
}

impl PostSplitResponse {
    pub fn with_promises(promises: Vec<BlindedSignature>) -> Self {
        Self { promises }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct CashuErrorResponse {
    pub code: u64,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use crate::primitives::PostSplitResponse;

    #[test]
    fn test_serialize_empty_split_response() -> anyhow::Result<()> {
        let response = PostSplitResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"promises\":[]}");
        Ok(())
    }
}
