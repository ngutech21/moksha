use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MintQuery {
    pub amount: Option<u64>,
    pub payment_hash: Option<String>,
}
