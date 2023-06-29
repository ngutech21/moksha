use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMintQuery {
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostMintQuery {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invoice {
    pub amount: u64,
    pub payment_request: String,
}

impl Invoice {
    pub fn new(amount: u64, payment_request: String) -> Self {
        Self {
            amount,
            payment_request,
        }
    }
}
