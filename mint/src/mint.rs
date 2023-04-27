use cashurs_core::model::{BlindedSignature, MintKeyset, Proofs};

use crate::{database::Database, error::CashuMintError, lightning::Lightning};

#[derive(Clone)]
pub struct Mint {
    pub lightning: Lightning,
    pub keyset: MintKeyset,
    pub db: Database,
}

impl Mint {
    pub fn new(secret: String, lightning: Lightning, db_path: String) -> Self {
        Self {
            lightning,
            keyset: MintKeyset::new(secret),
            db: Database::new(db_path),
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

        self.db.write_used_proofs(proofs.clone());
        // TODO check invoice

        let result = self.lightning.pay_invoice(payment_request).await?;

        Ok((true, result.payment_hash, vec![]))
    }
}
