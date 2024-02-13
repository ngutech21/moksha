use mokshamint::{config::MintConfig, mint::MintBuilder};
use std::{env, fmt};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let app_env = match env::var("MINT_APP_ENV") {
        Ok(v) if v.trim() == "dev" => AppEnv::Dev,
        _ => AppEnv::Prod,
    };

    println!("Running in {app_env} mode");

    if app_env == AppEnv::Dev {
        match dotenvy::dotenv() {
            Ok(path) => println!(".env read successfully from {}", path.display()),
            Err(e) => panic!("Could not load .env file: {e}"),
        };
    }

    let MintConfig {
        privatekey,
        derivation_path,
        info,
        lightning_fee,
        server,
        database,
        btconchain_backend,
        lightning_backend,
    } = MintConfig::read_config_with_defaults();

    let mint = MintBuilder::new()
        .with_mint_info(Some(info))
        .with_server(Some(server))
        .with_private_key(privatekey)
        .with_derivation_path(derivation_path)
        .with_db(database)
        .with_lightning(lightning_backend.expect("lightning not set"))
        .with_btc_onchain(btconchain_backend)
        .with_fee(Some(lightning_fee))
        .build()
        .await;

    mokshamint::server::run_server(mint?).await
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Prod,
}

impl fmt::Display for AppEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Prod => write!(f, "prod"),
        }
    }
}
