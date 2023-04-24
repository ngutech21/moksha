use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct RequestMintResponse {
    pub pr: String,
    pub hash: String,
}

#[derive(Debug, Deserialize)]
pub struct MintQuery {
    pub amount: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Keyset {
    pub keysets: Vec<String>,
}
