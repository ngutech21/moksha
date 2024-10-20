use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invoice {
    pub amount: u64,
    pub payment_request: String,
}

impl Invoice {
    pub const fn new(amount: u64, payment_request: String) -> Self {
        Self {
            amount,
            payment_request,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceResult {
    pub payment_hash: Vec<u8>,
    pub payment_request: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayInvoiceResult {
    pub payment_hash: String,
    /// total fees in sat
    pub total_fees: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceParams {
    pub amount: u64,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
}
