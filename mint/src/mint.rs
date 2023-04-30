use cashurs_core::{
    dhke::Dhke,
    model::{BlindedSignature, MintKeyset, Proofs},
};

use crate::{database::Database, error::CashuMintError, lightning::Lightning};

#[derive(Clone)]
pub struct Mint {
    pub lightning: Lightning,
    pub keyset: MintKeyset,
    pub db: Database,
    pub dhke: Dhke,
}

impl Mint {
    pub fn new(secret: String, lightning: Lightning, db_path: String) -> Self {
        Self {
            lightning,
            keyset: MintKeyset::new(secret),
            db: Database::new(db_path),
            dhke: Dhke::new(),
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

        let proofs_amount = proofs.get_total_amount();

        // TODO verify proofs

        // TODO check for fees
        let amount_msat = invoice
            .amount_milli_satoshis()
            .expect("Invoice amount is missing");

        if amount_msat < (proofs_amount / 1000) {
            return Err(CashuMintError::InvoiceAmountTooLow(format!(
                "Invoice amount is too low: {}",
                amount_msat
            )));
        }

        self.db.add_used_proofs(proofs.clone())?;
        // TODO check invoice

        let result = self.lightning.pay_invoice(payment_request).await?;

        Ok((true, result.payment_hash, vec![]))
    }
}
