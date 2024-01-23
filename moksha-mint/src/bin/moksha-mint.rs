use mokshamint::{
    config::{DatabaseConfig, LightningFeeConfig, MintInfoConfig, ServerConfig},
    lightning::{
        AlbyLightningSettings, LightningType, LnbitsLightningSettings, LndLightningSettings,
        StrikeLightningSettings,
    },
    mint::MintBuilder,
};
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

    // TODO move to config module
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
        .from_env::<MintInfoConfig>()
        .expect("Please provide mint info");

    let fee_config = LightningFeeConfig::from_env();
    let server_config = ServerConfig::from_env();
    let db_config = DatabaseConfig::from_env();

    let mint = MintBuilder::new()
        .with_mint_info(mint_info_settings)
        .with_server(server_config)
        .with_private_key(get_env("MINT_PRIVATE_KEY"))
        .with_db(db_config)
        .with_lightning(ln_type)
        .with_fee(fee_config)
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

fn get_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} not found", key))
}
