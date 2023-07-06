use anyhow::Result;
use fedimint_client::module::gen::DynClientModuleGen;
use fedimint_client::module::gen::{ClientModuleGenRegistry, IClientModuleGen};
use fedimint_client::secret::PlainRootSecretStrategy;
use fedimint_client::ClientBuilder;
use fedimint_core::api::GlobalFederationApi;
use fedimint_core::config::load_from_file;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::task::TaskGroup;
use fedimint_core::Amount;
use fedimint_core::{
    api::{IGlobalFederationApi, WsClientConnectInfo, WsFederationApi},
    config::ClientConfig,
};
use fedimint_ln_client::{LightningClientExt, LightningClientGen, LnReceiveState};
use fedimint_mint_client::{MintClientGen, MintClientModule};
use fedimint_wallet_client::WalletClientGen;
use std::fs::create_dir_all;
use std::fs::File;
use std::path::PathBuf;
use std::vec;
use std::{str::FromStr, sync::Arc};

use futures::StreamExt;

#[derive(Clone)]
pub struct FedimintWallet {
    client: fedimint_client::Client,
}

impl FedimintWallet {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: Self::create_client().await?,
        })
    }

    pub async fn connect(connect: &str) -> anyhow::Result<()> {
        println!("Connecting to {}", connect);
        let workdir = Self::workdir()?;
        let connect_obj: WsClientConnectInfo = WsClientConnectInfo::from_str(connect)?;
        let api = Arc::new(WsFederationApi::from_connect_info(&[connect_obj.clone()]))
            as Arc<dyn IGlobalFederationApi + Send + Sync + 'static>;
        let cfg: ClientConfig = api.download_client_config(&connect_obj).await?;
        create_dir_all(workdir.clone())?;
        let cfg_path = workdir.join("client.json");
        let writer = File::options()
            .create_new(true)
            .write(true)
            .open(cfg_path)?;
        serde_json::to_writer_pretty(writer, &cfg)?;
        Ok(())
    }

    pub async fn mint(&self, amount: u64) -> anyhow::Result<LnReceiveState> {
        let workdir = Self::workdir()?;
        println!("Workdir: {:?}", workdir);
        println!("Minting {} tokens", amount);
        self.client.select_active_gateway().await?;

        let (operation_id, invoice) = self
            .client
            .create_bolt11_invoice(Amount::from_sats(amount), "test".to_owned(), None)
            .await?;
        println!("Invoice: {}", invoice);

        let mut updates = self
            .client
            .subscribe_ln_receive(operation_id)
            .await?
            .into_stream();
        while let Some(update) = updates.next().await {
            match update {
                LnReceiveState::Claimed => {
                    return Ok(LnReceiveState::Claimed);
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
        Ok(summary.total_amount().msats * 1_000)
    }

    fn workdir() -> anyhow::Result<std::path::PathBuf> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
    }

    async fn create_client() -> Result<fedimint_client::Client> {
        let module_gens = ClientModuleGenRegistry::from(vec![
            DynClientModuleGen::from(LightningClientGen),
            DynClientModuleGen::from(MintClientGen),
            DynClientModuleGen::from(WalletClientGen::default()),
        ]);

        let config = Self::load_config()?;
        Self::load_decoders(&config, &module_gens);
        Self::build_client(&config, &module_gens).await
    }

    fn load_config() -> anyhow::Result<ClientConfig> {
        let cfg_path = Self::workdir()?.join("client.json");
        load_from_file(&cfg_path)
    }

    async fn build_client(
        cfg: &ClientConfig,
        module_gens: &ClientModuleGenRegistry,
    ) -> anyhow::Result<fedimint_client::Client> {
        let mut tg = TaskGroup::new();
        let db = Self::load_db().await?;

        let mut client_builder = ClientBuilder::default();
        client_builder.with_module_gens(module_gens.clone());
        client_builder.with_primary_module(1);
        client_builder.with_config(cfg.clone());
        client_builder.with_database(db);
        client_builder
            .build::<PlainRootSecretStrategy>(&mut tg)
            .await
    }

    async fn load_db() -> anyhow::Result<fedimint_sqlite::SqliteDb> {
        let db_path = Self::workdir()?.join("client.db");
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
