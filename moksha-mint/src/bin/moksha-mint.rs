use mokshamint::{
    info::MintInfoSettings,
    lightning::{
        AlbyLightningSettings, LightningType, LnbitsLightningSettings, LndLightningSettings,
        StrikeLightningSettings,
    },
    MintBuilder,
};
use std::{env, fmt, net::SocketAddr, path::PathBuf};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let app_env = match env::var("MINT_APP_ENV") {
        Ok(v) if v == "prod" => AppEnv::Prod,
        _ => AppEnv::Dev,
    };

    println!("Running in {app_env} mode");

    if app_env == AppEnv::Dev {
        match dotenvy::dotenv() {
            Ok(path) => println!(".env read successfully from {}", path.display()),
            Err(e) => panic!("Could not load .env file: {e}"),
        };
    }

    let host_port: SocketAddr = env::var("MINT_HOST_PORT")
        .unwrap_or_else(|_| "[::]:3338".to_string())
        .parse()?;

    // read optional env var
    // MINT_API_PREFIX

    let api_prefix = env::var("MINT_API_PREFIX").ok();

    let ln_backend = get_env("MINT_LIGHTNING_BACKEND");
    let ln_type = match ln_backend.as_str() {
        "Lnbits" => {
            let lnbits_settings = envy::prefixed("LNBITS_")
                .from_env::<LnbitsLightningSettings>()
                .expect("Please provide lnbits info");
            LightningType::Lnbits(lnbits_settings)
        }
        "Lnd" => {
            let lnd_settings = envy::prefixed("LND_")
                .from_env::<LndLightningSettings>()
                .expect("Please provide lnd info");
            LightningType::Lnd(lnd_settings)
        }
        "Alby" => {
            let alby_settings = envy::prefixed("ALBY_")
                .from_env::<AlbyLightningSettings>()
                .expect("Please provide alby info");
            LightningType::Alby(alby_settings)
        }
        "Strike" => {
            let strike_settings = envy::prefixed("STRIKE_")
                .from_env::<StrikeLightningSettings>()
                .expect("Please provide strike info");
            LightningType::Strike(strike_settings)
        }
        _ => panic!(
            "env MINT_LIGHTNING_BACKEND not found or invalid values. Valid values are Lnbits, Lnd, Alby, and Strike"
        ),
    };

    let mint_info_settings = envy::prefixed("MINT_INFO_")
        .from_env::<MintInfoSettings>()
        .expect("Please provide mint info");

    let mint = MintBuilder::new()
        .with_mint_info(mint_info_settings)
        .with_private_key(get_env("MINT_PRIVATE_KEY"))
        .with_db(get_env("MINT_DB_PATH"))
        .with_lightning(ln_type)
        .with_fee(
            get_env("LIGHTNING_FEE_PERCENT").parse()?,
            get_env("LIGHTNING_RESERVE_FEE_MIN").parse()?,
        )
        .build()
        .await;

    let serve_wallet_path = env::var("MINT_SERVE_WALLET_PATH");
    let serve_wallet_path = match serve_wallet_path {
        Ok(value) => Some(PathBuf::from(value)),
        Err(_) => None,
    };

    mokshamint::run_server(mint?, host_port, serve_wallet_path, api_prefix).await
}

#[derive(Debug, PartialEq)]
pub enum AppEnv {
    Dev,
    Prod,
}

impl fmt::Display for AppEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppEnv::Dev => write!(f, "dev"),
            AppEnv::Prod => write!(f, "prod"),
        }
    }
}

fn get_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} not found", key))
}
