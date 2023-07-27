use anyhow::Result;
use fedimint_client::module::gen::DynClientModuleGen;
use fedimint_client::module::gen::{ClientModuleGenRegistry, IClientModuleGen};
use fedimint_client::secret::PlainRootSecretStrategy;
use fedimint_client::sm::OperationId;
use fedimint_client::ClientBuilder;
use fedimint_core::api::GlobalFederationApi;
use fedimint_core::config::load_from_file;
use fedimint_core::encoding::Encodable;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::task::TaskGroup;
use fedimint_core::{
    api::{IGlobalFederationApi, WsClientConnectInfo, WsFederationApi},
    config::ClientConfig,
};
use fedimint_core::{Amount, TieredMulti};
use fedimint_ln_client::{
    LightningClientExt, LightningClientGen, LnPayState, LnReceiveState, PayType,
};
use fedimint_mint_client::{
    parse_ecash, MintClientExt, MintClientGen, MintClientModule, SpendableNote,
};
use fedimint_wallet_client::WalletClientGen;
use lightning_invoice::Invoice;
use std::fs::create_dir_all;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::vec;
use std::{str::FromStr, sync::Arc};

use futures::StreamExt;

const CLIENT_CFG: &str = "client.json";

#[derive(Clone)]
pub struct FedimintWallet {
    client: fedimint_client::Client,
}

impl FedimintWallet {
    pub async fn new(workdir: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            client: Self::create_client(&workdir).await?,
        })
    }

    pub async fn connect(workdir: PathBuf, connect: &str) -> anyhow::Result<()> {
        let connect_obj: WsClientConnectInfo = WsClientConnectInfo::from_str(connect)?;
        let api = Arc::new(WsFederationApi::from_connect_info(&[connect_obj.clone()]))
            as Arc<dyn IGlobalFederationApi + Send + Sync + 'static>;
        let cfg: ClientConfig = api.download_client_config(&connect_obj).await?;
        create_dir_all(workdir.clone())?;
        let cfg_path = workdir.join(CLIENT_CFG);
        let writer = File::options()
            .create_new(true)
            .write(true)
            .open(cfg_path)?;
        serde_json::to_writer_pretty(writer, &cfg)?;
        Ok(())
    }

    pub async fn get_mint_payment_request(&self, amount: u64) -> anyhow::Result<(String, Invoice)> {
        self.client.select_active_gateway().await?;

        let (operation_id, invoice) = self
            .client
            .create_bolt11_invoice(Amount::from_sats(amount), "test".to_owned(), None)
            .await?;
        Ok((operation_id.to_string(), invoice))
    }

    pub async fn pay_ln_invoice(&self, invoice: String) -> anyhow::Result<bool> {
        self.client.select_active_gateway().await?;

        let ln_invoice = Invoice::from_str(&invoice)?;

        let (pay_types, _) = self.client.pay_bolt11_invoice(ln_invoice).await?;
        let PayType::Lightning(operation_id) = pay_types else { unreachable!("paying invoice over lightning"); };
        let mut updates = self
            .client
            .subscribe_ln_pay(operation_id)
            .await?
            .into_stream();

        loop {
            match updates.next().await {
                // FIXME return enum
                Some(LnPayState::Success { preimage: _ }) => {
                    return Ok(true);
                }
                Some(LnPayState::Refunded { gateway_error }) => {
                    return Err(anyhow::anyhow!("refunded {gateway_error}"));
                }
                None => return Err(anyhow::anyhow!("Lightning send failed")),
                _ => {}
            }
        }
    }

    pub async fn receive_token(&self, tokens: String) -> anyhow::Result<u64> {
        let notes = parse_ecash(&tokens)?;
        let total_amount = notes.total_amount().msats / 1_000;

        let operation_id = self.client.reissue_external_notes(notes, ()).await?;
        let mut updates = self
            .client
            .subscribe_reissue_external_notes(operation_id)
            .await
            .unwrap()
            .into_stream();

        while let Some(update) = updates.next().await {
            if let fedimint_mint_client::ReissueExternalNotesState::Failed(e) = update {
                return Err(anyhow::Error::msg(format!("Reissue failed: {e}")));
            }
        }
        Ok(total_amount)
    }

    pub fn serialize_ecash(c: &TieredMulti<SpendableNote>) -> String {
        let mut bytes = Vec::new();
        Encodable::consensus_encode(c, &mut bytes).expect("encodes correctly");
        base64::encode(&bytes)
    }

    pub async fn send_tokens(
        &self,
        min_amount: u64,
        try_cancel_after: Duration,
    ) -> anyhow::Result<String> {
        let (_, notes) = self
            .client
            .spend_notes(Amount::from_sats(min_amount), try_cancel_after, ())
            .await?;
        Ok(Self::serialize_ecash(&notes))
    }

    pub async fn mint(&self, operation_id: String) -> anyhow::Result<()> {
        self.client.select_active_gateway().await?;

        let mut updates = self
            .client
            .subscribe_ln_receive(OperationId::from_str(&operation_id)?)
            .await?
            .into_stream();
        while let Some(update) = updates.next().await {
            match update {
                LnReceiveState::Claimed => {
                    return Ok(());
                }
                LnReceiveState::Canceled { reason } => {
                    return Err(reason.into());
                }
                _ => {}
            }
        }
        Err(anyhow::anyhow!("Something went wrong"))
    }

    pub async fn balance(&self) -> anyhow::Result<u64> {
        let (mint_client, _) = self
            .client
            .get_first_module::<MintClientModule>(&fedimint_mint_client::KIND);
        let summary = mint_client
            .get_wallet_summary(
                &mut self
                    .client
                    .db()
                    .begin_transaction()
                    .await
                    .with_module_prefix(1),
            )
            .await;
        Ok(summary.total_amount().msats / 1_000)
    }

    async fn create_client(workdir: &Path) -> Result<fedimint_client::Client> {
        let module_gens = ClientModuleGenRegistry::from(vec![
            DynClientModuleGen::from(LightningClientGen),
            DynClientModuleGen::from(MintClientGen),
            DynClientModuleGen::from(WalletClientGen::default()),
        ]);

        let config = Self::load_config(workdir)?;
        Self::load_decoders(&config, &module_gens);
        Self::build_client(&config, &module_gens, workdir).await
    }

    fn load_config(workdir: &Path) -> anyhow::Result<ClientConfig> {
        let cfg_path = workdir.join(CLIENT_CFG);
        load_from_file(&cfg_path)
    }

    // FIXME: this is a hack
    pub fn is_initialized(workdir: &Path) -> bool {
        workdir.join(CLIENT_CFG).exists()
    }

    async fn build_client(
        cfg: &ClientConfig,
        module_gens: &ClientModuleGenRegistry,
        workdir: &Path,
    ) -> anyhow::Result<fedimint_client::Client> {
        let mut tg = TaskGroup::new();

        #[cfg(not(target_arch = "wasm32"))]
        let db = Self::load_db(workdir).await?;

        #[cfg(target_arch = "wasm32")]
        let db = fedimint_core::db::mem_impl::MemDatabase::default();
        // FIXME use different db for fedimint in wasm

        let mut client_builder = ClientBuilder::default();
        client_builder.with_module_gens(module_gens.clone());
        client_builder.with_primary_module(1);
        client_builder.with_config(cfg.clone());
        client_builder.with_database(db);
        client_builder
            .build::<PlainRootSecretStrategy>(&mut tg)
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn load_db(workdir: &Path) -> anyhow::Result<fedimint_sqlite::SqliteDb> {
        let db_path = workdir.join("client.db");
        fedimint_sqlite::SqliteDb::open(db_path.to_str().unwrap())
            .await
            .map_err(anyhow::Error::new)
    }

    fn load_decoders(
        cfg: &ClientConfig,
        module_gens: &ClientModuleGenRegistry,
    ) -> ModuleDecoderRegistry {
        ModuleDecoderRegistry::new(cfg.clone().modules.into_iter().filter_map(
            |(id, module_cfg)| {
                let kind = module_cfg.kind().clone();
                module_gens.get(&kind).map(|module_gen| {
                    (
                        id,
                        kind,
                        IClientModuleGen::decoder(AsRef::<dyn IClientModuleGen + 'static>::as_ref(
                            module_gen,
                        )),
                    )
                })
            },
        ))
    }
}
