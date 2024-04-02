use std::{collections::HashSet, sync::Arc, vec};

use moksha_core::{
    amount::Amount,
    blind::{BlindedMessage, BlindedSignature, TotalAmount},
    dhke::Dhke,
    keyset::MintKeyset,
    primitives::{BtcOnchainMeltQuote, PaymentMethod},
    proof::Proofs,
};
use sqlx::Transaction;
use tracing::instrument;

use crate::{
    btconchain::{lnd::LndBtcOnchain, BtcOnchain},
    config::{
        BtcOnchainConfig, BtcOnchainType, BuildParams, DatabaseConfig, LightningFeeConfig,
        MintConfig, MintInfoConfig, ServerConfig, TracingConfig,
    },
    database::{postgres::PostgresDB, Database},
    error::MokshaMintError,
    lightning::{
        alby::AlbyLightning, lnbits::LnbitsLightning, lnd::LndLightning, strike::StrikeLightning,
        Lightning, LightningType,
    },
    model::Invoice,
};

use crate::lightning::cln::ClnLightning;

#[derive(Clone)]
pub struct Mint<DB: Database = PostgresDB> {
    pub lightning: Arc<dyn Lightning + Send + Sync>,
    pub lightning_type: LightningType,
    // FIXME remove after v1 api release
    pub keyset_legacy: MintKeyset,
    pub keyset: MintKeyset,
    pub db: DB,
    pub dhke: Dhke,
    pub onchain: Option<Arc<dyn BtcOnchain + Send + Sync>>,
    pub config: MintConfig,
    pub build_params: BuildParams,
}

impl<DB> Mint<DB>
where
    DB: Database,
{
    pub fn new(
        lightning: Arc<dyn Lightning + Send + Sync>,
        lightning_type: LightningType,
        db: DB,
        config: MintConfig,
        build_params: BuildParams,
        onchain: Option<Arc<dyn BtcOnchain + Send + Sync>>,
    ) -> Self {
        Self {
            lightning,
            lightning_type,
            keyset_legacy: MintKeyset::legacy_new(
                // FIXME
                &config.privatekey.clone(),
                &config.derivation_path.clone().unwrap_or_default(),
            ),
            keyset: MintKeyset::new(
                &config.privatekey.clone(),
                &config.derivation_path.clone().unwrap_or_default(),
            ),
            db,
            dhke: Dhke::new(),
            config,
            onchain,
            build_params,
        }
    }

    pub fn fee_reserve(&self, amount_msat: u64) -> u64 {
        let fee_percent = self.config.lightning_fee.fee_percent as f64 / 100.0;
        let fee_reserve = (amount_msat as f64 * fee_percent) as u64;
        std::cmp::max(fee_reserve, self.config.lightning_fee.fee_reserve_min)
    }

    pub fn create_blinded_signatures(
        &self,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset, // FIXME refactor keyset management
    ) -> Result<Vec<BlindedSignature>, MokshaMintError> {
        let promises = blinded_messages
            .iter()
            .map(|blinded_msg| {
                let private_key = keyset.private_keys.get(&blinded_msg.amount).unwrap(); // FIXME unwrap
                let blinded_sig = self.dhke.step2_bob(blinded_msg.b_, private_key).unwrap(); // FIXME unwrap
                BlindedSignature {
                    id: Some(keyset.keyset_id.clone()),
                    amount: blinded_msg.amount,
                    c_: blinded_sig,
                }
            })
            .collect::<Vec<BlindedSignature>>();
        Ok(promises)
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn create_invoice(
        &self,
        key: String,
        amount: u64,
    ) -> Result<(String, String), MokshaMintError> {
        let mut tx = self.db.begin_tx().await?;
        let pr = self.lightning.create_invoice(amount).await?.payment_request;
        self.db
            .add_pending_invoice(&mut tx, key.clone(), &Invoice::new(amount, pr.clone()))
            .await?;
        tx.commit().await?;
        Ok((pr, key))
    }

    #[instrument(level = "debug", skip(self, outputs, keyset), err)]
    pub async fn mint_tokens(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        payment_method: PaymentMethod,
        key: String,
        outputs: &[BlindedMessage],
        keyset: &MintKeyset,
        return_error: bool,
    ) -> Result<Vec<BlindedSignature>, MokshaMintError> {
        // FIXME refactor (split up in multiple functions)
        if payment_method == PaymentMethod::Bolt11 {
            let mut tx = self.db.begin_tx().await?;
            let invoice = self.db.get_pending_invoice(&mut tx, key.clone()).await?;

            let is_paid = self
                .lightning
                .is_invoice_paid(invoice.payment_request.clone())
                .await?;

            // FIXME remove after legacy api is removed
            if return_error && !is_paid {
                return Err(MokshaMintError::InvoiceNotPaidYet);
            }

            self.db.delete_pending_invoice(&mut tx, key).await?;
            tx.commit().await?;
        }
        self.create_blinded_signatures(outputs, keyset)
    }

    fn has_duplicate_pubkeys(outputs: &[BlindedMessage]) -> bool {
        let mut uniq = HashSet::new();
        !outputs.iter().all(move |x| uniq.insert(x.b_))
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn swap(
        &self,
        proofs: &Proofs,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset,
    ) -> Result<Vec<BlindedSignature>, MokshaMintError> {
        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        if Self::has_duplicate_pubkeys(blinded_messages) {
            return Err(MokshaMintError::SwapHasDuplicatePromises);
        }

        let sum_proofs = proofs.total_amount();

        let promises = self.create_blinded_signatures(blinded_messages, keyset)?;
        let amount_promises = promises.total_amount();
        if sum_proofs != amount_promises {
            return Err(MokshaMintError::SwapAmountMismatch(format!(
                "Split amount mismatch: {sum_proofs} != {amount_promises}"
            )));
        }

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;
        Ok(promises)
    }

    #[instrument(level = "debug", skip(self, proofs, blinded_messages, keyset), err)]
    pub async fn melt_bolt11(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        payment_request: String,
        fee_reserve: u64,
        proofs: &Proofs,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset,
    ) -> Result<(bool, String, Vec<BlindedSignature>), MokshaMintError> {
        let invoice = self
            .lightning
            .decode_invoice(payment_request.clone())
            .await?;

        let proofs_amount = proofs.total_amount();

        // TODO verify proofs

        self.check_used_proofs(tx, proofs).await?;

        // TODO check for fees
        let amount_msat = invoice
            .amount_milli_satoshis()
            .expect("Invoice amount is missing");

        if amount_msat < (proofs_amount / 1_000) {
            return Err(MokshaMintError::InvoiceAmountTooLow(format!(
                "Invoice amount is too low: {amount_msat}",
            )));
        }

        // TODO check invoice

        let result = self.lightning.pay_invoice(payment_request).await?;
        self.db.add_used_proofs(tx, proofs).await?;

        let change = if fee_reserve > 0 {
            let return_fees = Amount(fee_reserve - result.total_fees).split();

            if (return_fees.len()) > blinded_messages.len() {
                // FIXME better handle case when there are more fees than blinded messages
                vec![]
            } else {
                let out: Vec<_> = blinded_messages[0..return_fees.len()]
                    .iter()
                    .zip(return_fees.into_iter())
                    .map(|(message, fee)| BlindedMessage {
                        amount: fee,
                        ..message.clone()
                    })
                    .collect();

                self.create_blinded_signatures(&out, keyset)?
            }
        } else {
            vec![]
        };
        Ok((true, result.payment_hash, change))
    }

    pub async fn check_used_proofs(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        proofs: &Proofs,
    ) -> Result<(), MokshaMintError> {
        let used_proofs = self.db.get_used_proofs(tx).await?.proofs();
        for used_proof in used_proofs {
            if proofs.proofs().contains(&used_proof) {
                return Err(MokshaMintError::ProofAlreadyUsed(format!("{used_proof:?}")));
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip(self, proofs), err)]
    pub async fn melt_onchain(
        &self,
        quote: &BtcOnchainMeltQuote,
        proofs: &Proofs,
    ) -> Result<String, MokshaMintError> {
        let proofs_amount = proofs.total_amount();

        if proofs_amount < quote.amount {
            return Err(MokshaMintError::NotEnoughTokens(quote.amount));
        }

        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        let send_response = self
            .onchain
            .as_ref()
            .expect("onchain backend not set")
            .send_coins(&quote.address, quote.amount, quote.fee_sat_per_vbyte)
            .await?;

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;

        Ok(send_response.txid)
    }
}

#[derive(Debug, Default)]
pub struct MintBuilder {
    private_key: Option<String>,
    derivation_path: Option<String>,
    lightning_type: Option<LightningType>,

    db_config: Option<DatabaseConfig>,
    fee_config: Option<LightningFeeConfig>,
    mint_info_settings: Option<MintInfoConfig>,
    server_config: Option<ServerConfig>,
    btc_onchain_config: Option<BtcOnchainConfig>,
    tracing_config: Option<TracingConfig>,
}

impl MintBuilder {
    pub fn new() -> Self {
        MintBuilder {
            private_key: None,
            derivation_path: None,
            lightning_type: None,
            db_config: None,
            fee_config: None,
            mint_info_settings: None,
            server_config: None,
            btc_onchain_config: None,
            tracing_config: None,
        }
    }

    pub fn with_db(mut self, db_config: Option<DatabaseConfig>) -> Self {
        self.db_config = db_config;
        self
    }

    pub fn with_mint_info(mut self, mint_info: Option<MintInfoConfig>) -> Self {
        self.mint_info_settings = mint_info;
        self
    }

    pub fn with_server(mut self, server_config: Option<ServerConfig>) -> Self {
        self.server_config = server_config;
        self
    }

    pub fn with_private_key(mut self, private_key: String) -> Self {
        self.private_key = Some(private_key);
        self
    }

    pub fn with_derivation_path(mut self, derivation_path: Option<String>) -> Self {
        self.derivation_path = derivation_path;
        self
    }

    pub fn with_lightning(mut self, lightning: LightningType) -> Self {
        self.lightning_type = Some(lightning);
        self
    }

    pub const fn with_fee(mut self, fee_config: Option<LightningFeeConfig>) -> Self {
        self.fee_config = fee_config;
        self
    }

    pub fn with_btc_onchain(mut self, btc_onchain_config: Option<BtcOnchainConfig>) -> Self {
        self.btc_onchain_config = btc_onchain_config;
        self
    }

    pub fn with_tracing(mut self, tracing_config: Option<TracingConfig>) -> Self {
        self.tracing_config = tracing_config;
        self
    }

    pub async fn build(self) -> Result<Mint<PostgresDB>, MokshaMintError> {
        let ln: Arc<dyn Lightning + Send + Sync> = match self.lightning_type.clone() {
            Some(LightningType::Lnbits(lnbits_settings)) => Arc::new(LnbitsLightning::new(
                lnbits_settings.admin_key.expect("LNBITS_ADMIN_KEY not set"),
                lnbits_settings.url.expect("LNBITS_URL not set"),
            )),
            Some(LightningType::Alby(alby_settings)) => Arc::new(AlbyLightning::new(
                alby_settings.api_key.expect("ALBY_API_KEY not set"),
            )),
            Some(LightningType::Strike(strike_settings)) => Arc::new(StrikeLightning::new(
                strike_settings.api_key.expect("STRIKE_API_KEY not set"),
            )),
            Some(LightningType::Cln(set)) => Arc::new(
                ClnLightning::new(
                    set.grpc_host.expect("CLN_GRPC_HOST not set"),
                    &set.client_cert.expect("CLN_CLIENT_CERT not set"),
                    &set.client_key.expect("CLN_CLIENT_KEY not set"),
                    &set.ca_cert.expect("CLN_CA_CERT not set"),
                )
                .await?,
            ),
            Some(LightningType::Lnd(lnd_settings)) => Arc::new(
                LndLightning::new(
                    lnd_settings.grpc_host.expect("LND_GRPC_HOST not set"),
                    &lnd_settings
                        .tls_cert_path
                        .expect("LND_TLS_CERT_PATH not set"),
                    &lnd_settings
                        .macaroon_path
                        .expect("LND_MACAROON_PATH not set"),
                )
                .await?,
            ),
            None => panic!("Lightning backend not set"),
        };

        let lnd_onchain: Option<Arc<dyn BtcOnchain + Send + Sync>> =
            match self.btc_onchain_config.clone() {
                Some(BtcOnchainConfig {
                    onchain_type: Some(BtcOnchainType::Lnd(cfg)),
                    ..
                }) => Some(Arc::new(
                    LndBtcOnchain::new(
                        cfg.grpc_host.expect("LND_GRPC_HOST not found"),
                        &cfg.tls_cert_path.expect("LND_TLS_CERT_PATH not found"),
                        &cfg.macaroon_path.expect("LND_MACAROON_PATH not found"),
                    )
                    .await?,
                )),
                _ => None,
            };
        let db_config = self.db_config.expect("db-config not set");
        let db = PostgresDB::new(&db_config).await?;
        db.migrate().await;

        Ok(Mint::new(
            ln,
            self.lightning_type
                .clone()
                .expect("Lightning backend not set"),
            db,
            // FIXME simplify config creation
            MintConfig::new(
                self.private_key.expect("private-key not set"),
                self.derivation_path,
                self.mint_info_settings.unwrap_or_default(),
                self.fee_config.expect("fee-config not set"),
                self.server_config.unwrap_or_default(),
                db_config,
                self.btc_onchain_config,
                self.lightning_type,
                self.tracing_config,
            ),
            BuildParams::from_env(),
            lnd_onchain,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::btconchain::MockBtcOnchain;
    use crate::config::{DatabaseConfig, MintConfig};
    use crate::database::postgres::PostgresDB;
    use crate::database::Database;
    use crate::error::MokshaMintError;
    use crate::lightning::error::LightningError;
    use crate::lightning::{LightningType, MockLightning};
    use crate::mint::Mint;
    use crate::model::{Invoice, PayInvoiceResult};
    use moksha_core::blind::{BlindedMessage, TotalAmount};
    use moksha_core::dhke;
    use moksha_core::primitives::PostSplitRequest;
    use moksha_core::proof::Proofs;
    use moksha_core::token::TokenV3;
    use std::str::FromStr;
    use std::sync::Arc;
    use testcontainers::clients::Cli;
    use testcontainers::RunnableImage;
    use testcontainers_modules::postgres::Postgres;

    #[tokio::test]
    async fn test_fee_reserve() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432)).await?,
            None,
        )
        .await?;
        let fee = mint.fee_reserve(10000);
        assert_eq!(4000, fee);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_blindsignatures() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432)).await?,
            None,
        )
        .await?;

        let blinded_messages = vec![BlindedMessage {
            amount: 8,
            b_: dhke::public_key_from_hex(
                "02634a2c2b34bec9e8a4aba4361f6bf202d7fa2365379b0840afe249a7a9d71239",
            ),
            id: "00ffd48b8f5ecf80".to_owned(),
        }];

        let result = mint.create_blinded_signatures(&blinded_messages, &mint.keyset_legacy)?;

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
    async fn test_mint_empty() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mut lightning = MockLightning::new();
        lightning.expect_is_invoice_paid().returning(|_| Ok(true));
        let mint = create_mint_from_mocks(
            create_mock_db_pending_invoice(node.get_host_port_ipv4(5432)).await?,
            Some(lightning),
        )
        .await?;

        let mut tx = mint.db.begin_tx().await?;
        let outputs = vec![];
        let result = mint
            .mint_tokens(
                &mut tx,
                moksha_core::primitives::PaymentMethod::Bolt11,
                "somehash".to_string(),
                &outputs,
                &mint.keyset_legacy,
                true,
            )
            .await?;
        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_mint_valid() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mut lightning = MockLightning::new();
        lightning.expect_is_invoice_paid().returning(|_| Ok(true));
        let mint = create_mint_from_mocks(
            create_mock_db_pending_invoice(node.get_host_port_ipv4(5432)).await?,
            Some(lightning),
        )
        .await?;

        let outputs = create_blinded_msgs_from_fixture("blinded_messages_40.json".to_string())?;
        let mut tx = mint.db.begin_tx().await?;
        let result = mint
            .mint_tokens(
                &mut tx,
                moksha_core::primitives::PaymentMethod::Bolt11,
                "somehash".to_string(),
                &outputs,
                &mint.keyset_legacy,
                true,
            )
            .await?;
        assert_eq!(40, result.total_amount());
        Ok(())
    }

    #[tokio::test]
    async fn test_split_zero() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let blinded_messages = vec![];
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432)).await?,
            None,
        )
        .await?;

        let proofs = Proofs::empty();
        let result = mint
            .swap(&proofs, &blinded_messages, &mint.keyset_legacy)
            .await?;

        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_split_64_in_20() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432)).await?,
            None,
        )
        .await?;
        let request = create_request_from_fixture("post_split_request_64_20.json".to_string())?;

        let result = mint
            .swap(&request.proofs, &request.outputs, &mint.keyset_legacy)
            .await?;
        assert_eq!(result.total_amount(), 64);

        let prv_last = result.get(result.len() - 2).unwrap();
        let last = result.last().unwrap();

        assert_eq!(prv_last.amount, 4);
        assert_eq!(last.amount, 16);
        Ok(())
    }

    #[tokio::test]
    async fn test_split_duplicate_key() -> anyhow::Result<()> {
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432)).await?,
            None,
        )
        .await?;
        let request =
            create_request_from_fixture("post_split_request_duplicate_key.json".to_string())?;

        let result = mint
            .swap(&request.proofs, &request.outputs, &mint.keyset_legacy)
            .await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    /// melt 20 sats with 60 tokens and receive 40 tokens as change
    async fn test_melt_overpay() -> anyhow::Result<()> {
        use lightning_invoice::Bolt11Invoice as LNInvoice;
        let docker = Cli::default();
        let image = create_postgres_image();
        let node = docker.run(image);

        let mut lightning = MockLightning::new();

        lightning.expect_decode_invoice().returning(|_| {
            Ok(
                // 20 sat
                LNInvoice::from_str("lnbc200n1pj9eanxsp5agdl4rd0twdljpcgmg67dwj9mseu5m4lwfhslkws4uh4m5f5pcrqpp5lvspx676rykr64l02s97wjztcxe355qck0naydrsvvkqw42cc35sdq2f38xy6t5wvxqzjccqpjrzjq027t9tsc6jn5ve2k6gnn689unn8h239juuf9s3ce09aty6ed73t5z7nqsqqsygqqyqqqqqqqqqqqqgq9q9qyysgqs5msn4j9v53fq000zhw0gulkcx2dlnfdt953v2ur7z765jj3m0fx6cppkpjwntq5nsqm273u4eevva508pvepg8mh27sqcd29sfjr4cq255a40").expect("invalid invoice")
            )
        });
        lightning.expect_pay_invoice().returning(|_| {
            Ok(PayInvoiceResult {
                payment_hash: "hash".to_string(),
                total_fees: 2,
            })
            .map_err(|_err: LightningError| MokshaMintError::InvoiceNotFound("".to_string()))
        });

        let db = create_mock_db_empty(node.get_host_port_ipv4(5432)).await?;

        let mint = Mint::new(
            // "TEST_PRIVATE_KEY".to_string(),
            // "0/0/0/0".to_string(),
            Arc::new(lightning),
            LightningType::Lnbits(Default::default()),
            db,
            Default::default(),
            Default::default(),
            Some(Arc::new(MockBtcOnchain::default())),
        );

        let tokens = create_token_from_fixture("token_60.cashu".to_string())?;
        let invoice = "some invoice".to_string();
        let change =
            create_blinded_msgs_from_fixture("blinded_messages_blank_4000.json".to_string())?;

        let mut tx = mint.db.begin_tx().await?;
        let (paid, _payment_hash, change) = mint
            .melt_bolt11(
                &mut tx,
                invoice,
                4,
                &tokens.proofs(),
                &change,
                &mint.keyset_legacy,
            )
            .await?;

        assert!(paid);
        assert!(change.total_amount() == 2);
        Ok(())
    }

    // FIXME refactor helper functions
    fn create_token_from_fixture(fixture: String) -> Result<TokenV3, anyhow::Error> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{fixture}"))?;
        Ok(raw_token.trim().to_string().try_into()?)
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

    async fn create_mint_from_mocks(
        mock_db: PostgresDB,
        mock_ln: Option<MockLightning>,
    ) -> anyhow::Result<Mint> {
        let lightning = match mock_ln {
            Some(ln) => Arc::new(ln),
            None => Arc::new(MockLightning::new()),
        };

        Ok(Mint::new(
            lightning,
            LightningType::Lnbits(Default::default()),
            mock_db,
            MintConfig {
                privatekey: "TEST_PRIVATE_KEY".to_string(),
                derivation_path: Some("0/0/0/0".to_string()),
                ..Default::default()
            },
            Default::default(),
            Some(Arc::new(MockBtcOnchain::default())),
        ))
    }

    async fn create_mock_db_empty(port: u16) -> anyhow::Result<PostgresDB> {
        let connection_string =
            &format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = PostgresDB::new(&DatabaseConfig {
            db_url: connection_string.to_owned(),
            ..Default::default()
        })
        .await?;
        db.migrate().await;
        Ok(db)
    }

    async fn create_mock_db_pending_invoice(port: u16) -> anyhow::Result<PostgresDB> {
        let db = create_mock_db_empty(port).await?;

        let mut tx = db.begin_tx().await?;
        let invoice = Invoice{
            amount: 100,
            payment_request: "lnbcrt1u1pjgamjepp5cr2dzhcuy9tjwl7u45kxa9h02khvsd2a7f2x9yjxgst8trduld4sdqqcqzzsxqyz5vqsp5kaclwkq79ylef295qj7x6c9kvhaq6272ge4tgz7stlzv46csrzks9qyyssq9szxlvhh0uen2jmh07hp242nj5529wje3x5e434kepjzeqaq5hnsje8rzrl97s0j8cxxt3kgz5gfswrrchr45u8fq3twz2jjc029klqpd6jmgv".to_string(),
        };
        db.add_pending_invoice(&mut tx, "somehash".to_string(), &invoice)
            .await?;
        tx.commit().await?;
        Ok(db)
    }

    fn create_postgres_image() -> RunnableImage<Postgres> {
        let node = Postgres::default().with_host_auth();
        RunnableImage::from(node).with_tag("16.2-alpine")
    }
}
