use std::{collections::HashSet, sync::Arc};

use cashurs_core::{
    crypto,
    dhke::Dhke,
    model::{split_amount, BlindedMessage, BlindedSignature, MintKeyset, Proofs},
};

use crate::{database::Database, error::CashuMintError, lightning::Lightning, model::Invoice};

#[derive(Clone)]
pub struct Mint {
    pub lightning: Arc<dyn Lightning + Send + Sync>,
    pub keyset: MintKeyset,
    pub db: Arc<dyn Database + Send + Sync>,
    pub dhke: Dhke,
    pub lightning_fee_config: LightningFeeConfig,
}

#[derive(Clone, Debug)]
pub struct LightningFeeConfig {
    pub fee_percent: f32,
    pub fee_reserve_min: u64,
    // TODO check of fee_percent is in range
}

impl Default for LightningFeeConfig {
    fn default() -> Self {
        Self {
            fee_percent: 1.0,
            fee_reserve_min: 4000,
        }
    }
}

impl Mint {
    pub fn new(
        secret: String,
        derivation_path: String,
        lightning: Arc<dyn Lightning + Send + Sync>,
        db: Arc<dyn Database + Send + Sync>,
        lightning_fee_config: LightningFeeConfig,
    ) -> Self {
        Self {
            lightning,
            lightning_fee_config,
            keyset: MintKeyset::new(secret, derivation_path),
            db,
            dhke: Dhke::new(),
        }
    }

    pub fn fee_reserve(&self, amount_msat: u64) -> u64 {
        let fee_percent = self.lightning_fee_config.fee_percent as f64 / 100.0;
        let fee_reserve = (amount_msat as f64 * fee_percent) as u64;
        std::cmp::max(fee_reserve, self.lightning_fee_config.fee_reserve_min)
    }

    // TODO write tests for fee_reserve

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

    pub async fn create_invoice(&self, amount: u64) -> Result<(String, String), CashuMintError> {
        let pr = self.lightning.create_invoice(amount).await.payment_request;
        let key = crypto::generate_hash();
        self.db
            .add_pending_invoice(key.clone(), Invoice::new(amount, pr.clone()))?;
        Ok((pr, key))
    }

    pub async fn mint_tokens(
        &self,
        invoice_hash: String,
        outputs: Vec<BlindedMessage>,
    ) -> Result<Vec<BlindedSignature>, CashuMintError> {
        let invoice = self.db.get_pending_invoice(invoice_hash.clone())?;

        let is_paid = self
            .lightning
            .is_invoice_paid(invoice.payment_request.clone())
            .await?;

        if !is_paid {
            return Ok(vec![]);
        }

        self.db.remove_pending_invoice(invoice_hash)?;
        self.create_blinded_signatures(outputs).await
    }

    fn has_duplicate_pubkeys(outputs: &[BlindedMessage]) -> bool {
        let mut uniq = HashSet::new();
        !outputs.iter().all(move |x| uniq.insert(x.b_))
    }

    pub async fn split(
        &self,
        amount: u64,
        proofs: Proofs,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<(Vec<BlindedSignature>, Vec<BlindedSignature>), CashuMintError> {
        self.check_used_proofs(&proofs)?;

        if Self::has_duplicate_pubkeys(&blinded_messages) {
            return Err(CashuMintError::SplitHasDuplicatePromises);
        }

        let sum_proofs = proofs.get_total_amount();

        if amount > sum_proofs {
            return Err(CashuMintError::SplitAmountTooHigh);
        }
        let sum_first = split_amount(sum_proofs - amount).len();

        // TODO check: "split of promises is not as expected."

        let first_slice = blinded_messages[0..sum_first].to_vec();
        let first_sigs = self.create_blinded_signatures(first_slice).await?;
        let second_slice = blinded_messages[sum_first..blinded_messages.len()].to_vec();
        let second_sigs = self.create_blinded_signatures(second_slice).await?;

        let amount_first = self.get_amount(&first_sigs);
        let amount_second = self.get_amount(&second_sigs);

        if sum_proofs != (amount_first + amount_second) {
            return Err(CashuMintError::SplitAmountMismatch(format!(
                "Split amount mismatch: {sum_proofs} != {amount_first} + {amount_second}"
            )));
        }

        self.db.add_used_proofs(proofs)?;

        Ok((first_sigs, second_sigs))
    }

    fn get_amount(&self, blinded_sigs: &[BlindedSignature]) -> u64 {
        blinded_sigs
            .iter()
            .map(|blinded_sig| blinded_sig.amount)
            .sum()
    }

    pub async fn melt(
        &self,
        payment_request: String,
        proofs: Proofs,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<(bool, String, Vec<BlindedSignature>), CashuMintError> {
        let invoice = self
            .lightning
            .decode_invoice(payment_request.clone())
            .await?;

        let proofs_amount = proofs.get_total_amount();

        // TODO verify proofs

        self.check_used_proofs(&proofs)?;

        // TODO check for fees
        let amount_msat = invoice
            .amount_milli_satoshis()
            .expect("Invoice amount is missing");

        if amount_msat < (proofs_amount / 1000) {
            return Err(CashuMintError::InvoiceAmountTooLow(format!(
                "Invoice amount is too low: {amount_msat}",
            )));
        }

        self.db.add_used_proofs(proofs)?;
        // TODO check invoice

        let result = self.lightning.pay_invoice(payment_request).await?;

        let _remaining_amount = (amount_msat - (proofs_amount / 1000)) * 1000;

        // FIXME check if output amount matches remaining_amount
        let output = self.create_blinded_signatures(blinded_messages).await?;

        Ok((true, result.payment_hash, output))
    }

    pub fn check_used_proofs(&self, proofs: &Proofs) -> Result<(), CashuMintError> {
        let used_proofs = self.db.get_used_proofs()?.get_proofs();
        for used_proof in used_proofs {
            if proofs.get_proofs().contains(&used_proof) {
                return Err(CashuMintError::ProofAlreadyUsed(format!("{used_proof:?}")));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::lightning::MockLightning;
    use crate::mint::LightningFeeConfig;
    use crate::{database::MockDatabase, error::CashuMintError, Mint};
    use cashurs_core::dhke;
    use cashurs_core::model::{BlindedMessage, Tokens, TotalAmount};
    use cashurs_core::model::{PostSplitRequest, Proofs};
    use lnbits_rust::api::invoice::PayInvoiceResult;
    use std::str::FromStr;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_blindsignatures() -> anyhow::Result<()> {
        let mint = create_mint_from_mocks(None);

        let blinded_messages = vec![BlindedMessage {
            amount: 8,
            b_: dhke::public_key_from_hex(
                "02634a2c2b34bec9e8a4aba4361f6bf202d7fa2365379b0840afe249a7a9d71239",
            ),
        }];

        let result = mint.create_blinded_signatures(blinded_messages).await?;

        assert_eq!(1, result.len());
        assert_eq!(8, result[0].amount);
        assert_eq!(
            dhke::public_key_from_hex(
                "037074c4f53e326ee14ed67125f387d160e0e729351471b69ad41f7d5d21071e15"
            ),
            result[0].c_
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_split_zero() -> anyhow::Result<()> {
        let blinded_messages = vec![];
        let mint = create_mint_from_mocks(Some(create_mock_db_get_used_proofs()));

        let proofs = Proofs::empty();
        let (first, second) = mint.split(0, proofs, blinded_messages).await?;

        assert_eq!(first.len(), 0);
        assert_eq!(second.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_split_64_in_20() -> anyhow::Result<()> {
        let mint = create_mint_from_mocks(Some(create_mock_db_get_used_proofs()));
        let request = create_request_from_fixture("post_split_request_64_20.json".to_string())?;

        let (first, second) = mint.split(20, request.proofs, request.outputs).await?;

        first.total_amount();
        assert_eq!(first.total_amount(), 20);
        assert_eq!(second.total_amount(), 44);
        Ok(())
    }

    #[tokio::test]
    async fn test_split_64_in_64() -> anyhow::Result<()> {
        let mint = create_mint_from_mocks(Some(create_mock_db_get_used_proofs()));
        let request = create_request_from_fixture("post_split_request_64_20.json".to_string())?;

        let (first, second) = mint.split(64, request.proofs, request.outputs).await?;

        first.total_amount();
        assert_eq!(first.total_amount(), 64);
        assert_eq!(second.total_amount(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_split_amount_is_too_high() -> anyhow::Result<()> {
        let mint = create_mint_from_mocks(Some(create_mock_db_get_used_proofs()));
        let request = create_request_from_fixture("post_split_request_64_20.json".to_string())?;

        let result = mint.split(65, request.proofs, request.outputs).await;
        assert!(result.is_err());
        let _err = result.unwrap_err();
        assert!(matches!(CashuMintError::SplitAmountTooHigh, _err));

        Ok(())
    }

    #[tokio::test]
    async fn test_split_duplicate_key() -> anyhow::Result<()> {
        let mint = create_mint_from_mocks(Some(create_mock_db_get_used_proofs()));
        let request =
            create_request_from_fixture("post_split_request_duplicate_key.json".to_string())?;

        let result = mint.split(20, request.proofs, request.outputs).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    /// melt 20 sats with 60 tokens and receive 40 tokens as change
    async fn test_melt_overpay() -> anyhow::Result<()> {
        use lightning_invoice::Invoice as LNInvoice;

        let mut lightning = MockLightning::new();

        lightning.expect_decode_invoice().returning(|_| {
            Ok(
                // 20 sat
                LNInvoice::from_str("lnbc200n1pj9eanxsp5agdl4rd0twdljpcgmg67dwj9mseu5m4lwfhslkws4uh4m5f5pcrqpp5lvspx676rykr64l02s97wjztcxe355qck0naydrsvvkqw42cc35sdq2f38xy6t5wvxqzjccqpjrzjq027t9tsc6jn5ve2k6gnn689unn8h239juuf9s3ce09aty6ed73t5z7nqsqqsygqqyqqqqqqqqqqqqgq9q9qyysgqs5msn4j9v53fq000zhw0gulkcx2dlnfdt953v2ur7z765jj3m0fx6cppkpjwntq5nsqm273u4eevva508pvepg8mh27sqcd29sfjr4cq255a40").unwrap()
            )
        });
        lightning.expect_pay_invoice().returning(|_| {
            Ok(PayInvoiceResult {
                payment_hash: "hash".to_string(),
            })
        });

        let mint = Mint::new(
            "TEST_PRIVATE_KEY".to_string(),
            "0/0/0/0".to_string(),
            Arc::new(lightning),
            Arc::new(create_mock_db_get_used_proofs()),
            LightningFeeConfig::default(),
        );

        let tokens = create_token_from_fixture("token_60.cashu".to_string())?;
        let invoice = "some invoice".to_string();
        let change = create_blinded_msgs_from_fixture("blinded_messages_40.json".to_string())?;

        let (paid, _payment_hash, change) = mint.melt(invoice, tokens.get_proofs(), change).await?;

        assert!(paid);
        assert!(change.total_amount() == 40);
        println!("{:?}", change);
        Ok(())
    }

    // FIXME refactor helper functions
    fn create_token_from_fixture(fixture: String) -> Result<Tokens, anyhow::Error> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{fixture}"))?;
        Ok(Tokens::deserialize(raw_token.trim().to_string())?)
    }

    fn create_request_from_fixture(fixture: String) -> Result<PostSplitRequest, anyhow::Error> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{fixture}"))?;
        Ok(serde_json::from_str::<PostSplitRequest>(&raw_token)?)
    }

    fn create_blinded_msgs_from_fixture(
        fixture: String,
    ) -> Result<Vec<BlindedMessage>, anyhow::Error> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{fixture}"))?;
        Ok(serde_json::from_str::<Vec<BlindedMessage>>(&raw_token)?)
    }

    fn create_mint_from_mocks(mock_db: Option<MockDatabase>) -> Mint {
        let db = match mock_db {
            Some(db) => Arc::new(db),
            None => Arc::new(MockDatabase::new()),
        };

        let lightning = Arc::new(MockLightning::new());
        Mint::new(
            "TEST_PRIVATE_KEY".to_string(),
            "0/0/0/0".to_string(),
            lightning,
            db,
            LightningFeeConfig::default(),
        )
    }

    fn create_mock_db_get_used_proofs() -> MockDatabase {
        let mut mock_db = MockDatabase::new();
        mock_db
            .expect_get_used_proofs()
            .returning(|| Ok(Proofs::empty()));
        mock_db.expect_add_used_proofs().returning(|_| Ok(()));
        mock_db
    }
}
