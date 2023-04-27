use cashurs_core::model::{BlindedSignature, MintKeyset, Proofs};

use crate::{error::CashuMintError, lightning::Lightning};

#[derive(Clone)]
pub struct Mint {
    pub lightning: Lightning,
    pub keyset: MintKeyset,
}

impl Mint {
    pub fn new(secret: String, lightning: Lightning) -> Self {
        Self {
            lightning,
            keyset: MintKeyset::new(secret),
        }
    }

    pub async fn melt(
        &self,
        payment_request: String,
        proofs: Proofs,
    ) -> Result<(bool, String, Vec<BlindedSignature>), CashuMintError> {
        let invoice = self
            .lightning
            .decode_invoice(payment_request.clone())
            .await?;

        self.lightning.pay_invoice(payment_request).await?;

        Ok((true, "dummy preimage".to_string(), vec![]))
    }
}
