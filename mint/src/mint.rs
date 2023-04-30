use cashurs_core::{
    dhke::Dhke,
    model::{split_amount, BlindedMessage, BlindedSignature, MintKeyset, Proofs},
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

    pub async fn create_blinded_signatures(
        &self,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<Vec<BlindedSignature>, CashuMintError> {
        let promises = blinded_messages
            .iter()
            .map(|blinded_msg| {
                let private_key = self.keyset.private_keys.get(&blinded_msg.amount).unwrap(); // FIXME unwrap
                let blinded_sig = self.dhke.step2_bob(blinded_msg.b_, private_key).unwrap(); // FIXME unwrap
                BlindedSignature {
                    id: Some(self.keyset.keyset_id.clone()),
                    amount: blinded_msg.amount,
                    c_: blinded_sig,
                }
            })
            .collect::<Vec<BlindedSignature>>();
        Ok(promises)
    }

    pub async fn split(
        &self,
        amount: u64,
        proofs: Proofs,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<(Vec<BlindedSignature>, Vec<BlindedSignature>), CashuMintError> {
        let sum_proofs = proofs.get_total_amount();
        let sum_frst = split_amount(sum_proofs - amount).len();

        // TODO check: "split amount is higher than the total sum."
        // TODO check: "duplicate promises."
        // TODO check: "split of promises is not as expected."

        let first_slice = blinded_messages[0..sum_frst].to_vec();
        let first_sigs = self.create_blinded_signatures(first_slice).await?;
        let second_slice = blinded_messages[sum_frst..blinded_messages.len()].to_vec();
        let second_sigs = self.create_blinded_signatures(second_slice).await?;

        // TODO check: # verify amounts in produced proofs

        Ok((first_sigs, second_sigs))
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
