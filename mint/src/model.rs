use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MintQuery {
    pub amount: Option<u64>,
}
