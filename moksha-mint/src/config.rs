use std::{env, net::SocketAddr, path::PathBuf};

use crate::lightning::LndLightningSettings;

use moksha_core::primitives::{CurrencyUnit, Nut14, Nut15, PaymentMethod};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MintConfig {
    pub info: MintInfoConfig,
    pub build: BuildConfig,
    pub lightning_fee: LightningFeeConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub onchain: Option<OnchainConfig>,
}

impl MintConfig {
    pub fn new(
        info: MintInfoConfig,
        build: BuildConfig,
        lightning_fee: LightningFeeConfig,
        server: ServerConfig,
        database: DatabaseConfig,
        onchain: Option<OnchainConfig>,
    ) -> Self {
        Self {
            info,
            build,
            lightning_fee,
            server,
            database,
            onchain,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainConfig {
    pub onchain_type: OnchainType,
    pub min_confirmations: u8,
    pub min_amount: u64,
    pub max_amount: u64,
}

impl Default for OnchainConfig {
    fn default() -> Self {
        Self {
            onchain_type: OnchainType::Lnd(LndLightningSettings::default()),
            min_confirmations: 1,
            min_amount: 1_000,
            max_amount: 1_000_000,
        }
    }
}

impl OnchainConfig {
    pub fn from_env() -> Option<Self> {
        let onchain_type = OnchainType::from_env();

        println!("onchain_type: {:?}", onchain_type);

        onchain_type.as_ref()?;

        let def = OnchainConfig::default();

        Some(OnchainConfig {
            onchain_type: onchain_type.unwrap(),
            min_amount: env_or_default("MINT_ONCHAIN_BACKEND_MIN_AMOUNT", def.min_amount),
            max_amount: env_or_default("MINT_ONCHAIN_BACKEND_MAX_AMOUNT", def.max_amount),
            min_confirmations: env_or_default(
                "MINT_ONCHAIN_BACKEND_MIN_CONFIRMATIONS",
                def.min_confirmations,
            ),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnchainType {
    Lnd(LndLightningSettings),
}

impl OnchainType {
    pub fn from_env() -> Option<Self> {
        let onchain_type = env::var("MINT_ONCHAIN_BACKEND").ok();

        match onchain_type.as_deref() {
            None => None,
            Some("Lnd") => {
                // reuse lnd settings with prefix LND_
                let lnd_settings = envy::prefixed("LND_")
                    .from_env::<LndLightningSettings>()
                    .expect("Please provide lnd info");
                Some(OnchainType::Lnd(lnd_settings))
            }
            _ => {
                panic!("env MINT_ONCHAIN_BACKEND not found or invalid values. Valid values are Lnd")
            }
        }
    }
}

impl From<OnchainConfig> for Nut14 {
    fn from(settings: OnchainConfig) -> Self {
        Self {
            supported: true,
            payment_methods: vec![(PaymentMethod::Onchain, CurrencyUnit::Sat)],
            min_amount: settings.min_amount,
            max_amount: settings.max_amount,
        }
    }
}

impl From<OnchainConfig> for Nut15 {
    fn from(settings: OnchainConfig) -> Self {
        Self {
            supported: true,
            payment_methods: vec![(PaymentMethod::Onchain, CurrencyUnit::Sat)],
            min_amount: settings.min_amount,
            max_amount: settings.max_amount,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct DatabaseConfig {
    pub url: Option<String>,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        DatabaseConfig {
            url: env::var("MINT_DB_URL").ok(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ServerConfig {
    pub host_port: SocketAddr,
    pub serve_wallet_path: Option<PathBuf>,
    pub api_prefix: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host_port: "[::]:3338".to_string().parse().expect("invalid host port"),
            serve_wallet_path: None,
            api_prefix: None,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let server_config_default = ServerConfig::default();

        ServerConfig {
            host_port: env_or_default("MINT_HOST_PORT", server_config_default.host_port),
            serve_wallet_path: env::var("MINT_SERVE_WALLET_PATH").ok().map(PathBuf::from),
            api_prefix: env::var("MINT_API_PREFIX").ok(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MintInfoConfig {
    pub name: Option<String>,
    #[serde(default = "default_version")]
    pub version: bool,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
    // FIXME add missing fields for v1/info endpoint nut4/nut5 payment_methods, nut4 disabled flag
}

fn default_version() -> bool {
    true
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct BuildConfig {
    pub commit_hash: Option<String>,
    pub build_time: Option<String>,
    pub cargo_pkg_version: Option<String>,
}

impl BuildConfig {
    pub fn from_env() -> Self {
        Self {
            commit_hash: env::var("COMMITHASH").ok().map(|s| s.to_string()),
            build_time: env::var("BUILDTIME").ok().map(|s| s.to_string()),
            cargo_pkg_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }
    }

    pub fn full_version(&self) -> String {
        format!(
            "{}-{}",
            self.cargo_pkg_version
                .as_ref()
                .unwrap_or(&"unknown".to_string()),
            self.commit_hash.as_ref().unwrap_or(&"unknown".to_string())
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LightningFeeConfig {
    pub fee_percent: f32,
    pub fee_reserve_min: u64,
    // TODO check if fee_percent is in range
}

impl LightningFeeConfig {
    pub fn new(fee_percent: f32, fee_reserve_min: u64) -> Self {
        Self {
            fee_percent,
            fee_reserve_min,
        }
    }

    pub fn from_env() -> Self {
        let fee_config_default = LightningFeeConfig::default();

        LightningFeeConfig {
            fee_percent: env_or_default("LIGHTNING_FEE_PERCENT", fee_config_default.fee_percent),
            fee_reserve_min: env_or_default(
                "LIGHTNING_RESERVE_FEE_MIN",
                fee_config_default.fee_reserve_min,
            ),
        }
    }
}

impl From<(f32, u64)> for LightningFeeConfig {
    fn from(tuple: (f32, u64)) -> Self {
        Self {
            fee_percent: tuple.0,
            fee_reserve_min: tuple.1,
        }
    }
}

fn env_or_default<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

impl Default for LightningFeeConfig {
    fn default() -> Self {
        Self {
            fee_percent: 1.0,
            fee_reserve_min: 4000,
        }
    }
}
