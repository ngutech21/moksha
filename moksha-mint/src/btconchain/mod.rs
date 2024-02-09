use async_trait::async_trait;

use crate::error::MokshaMintError;

pub mod lnd;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait BtcOnchain: Send + Sync {
    async fn new_address(&self) -> Result<String, MokshaMintError>;
    async fn send_coins(
        &self,
        address: &str,
        amount: u64,
        sat_per_vbyte: u32,
    ) -> Result<SendCoinsResult, MokshaMintError>;

    async fn estimate_fee(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<EstimateFeeResult, MokshaMintError>;

    async fn is_paid(
        &self,
        address: &str,
        amount: u64,
        min_confirmations: u8,
    ) -> Result<bool, MokshaMintError>;

    async fn is_transaction_paid(&self, txid: &str) -> Result<bool, MokshaMintError>;
}

#[derive(Debug, Clone)]
pub struct EstimateFeeResult {
    pub fee_in_sat: u64,
    pub sat_per_vbyte: u32,
}

#[derive(Debug, Clone)]
pub struct SendCoinsResult {
    pub txid: String,
}
