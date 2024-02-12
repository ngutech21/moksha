use crate::{
    error::MokshaMintError,
    model::{CreateInvoiceResult, PayInvoiceResult},
};
use async_trait::async_trait;
use lightning_invoice::Bolt11Invoice as LNInvoice;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};

pub mod alby;
pub mod cln;
pub mod error;
pub mod lnbits;
pub mod lnd;
pub mod strike;

#[cfg(test)]
use mockall::automock;
use std::str::FromStr;

use self::lnd::LndLightningSettings;
use self::{
    alby::AlbyLightningSettings, cln::ClnLightningSettings, lnbits::LnbitsLightningSettings,
    strike::StrikeLightningSettings,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LightningType {
    Lnbits(LnbitsLightningSettings),
    Alby(AlbyLightningSettings),
    Strike(StrikeLightningSettings),
    Lnd(LndLightningSettings),
    Cln(ClnLightningSettings),
}

impl fmt::Display for LightningType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lnbits(settings) => write!(f, "Lnbits: {}", settings),
            Self::Alby(settings) => write!(f, "Alby: {}", settings),
            Self::Strike(settings) => write!(f, "Strike: {}", settings),
            Self::Lnd(settings) => write!(f, "Lnd: {}", settings),
            Self::Cln(settings) => write!(f, "Cln: {}", settings),
        }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Lightning: Send + Sync {
    async fn is_invoice_paid(&self, invoice: String) -> Result<bool, MokshaMintError>;
    async fn create_invoice(&self, amount: u64) -> Result<CreateInvoiceResult, MokshaMintError>;
    async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, MokshaMintError>;

    async fn decode_invoice(&self, payment_request: String) -> Result<LNInvoice, MokshaMintError> {
        LNInvoice::from_str(&payment_request)
            .map_err(|err| MokshaMintError::DecodeInvoice(payment_request, err))
    }
}
