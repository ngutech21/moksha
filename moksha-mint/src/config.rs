use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

use clap::Parser;
use moksha_core::primitives::{
    ContactInfoResponse, CurrencyUnit, Nut18, Nut19, PaymentMethod,
    PaymentMethodConfigBtcOnchainMelt, PaymentMethodConfigBtcOnchainMint,
};
use serde::{Deserialize, Serialize};

use crate::lightning::{
    alby::AlbyLightningSettings, cln::ClnLightningSettings, lnbits::LnbitsLightningSettings,
    lnd::LndLightningSettings, strike::StrikeLightningSettings, LightningType,
};

#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Opts {
    #[clap(long, env = "MINT_PRIVATE_KEY")]
    pub privatekey: String,
    #[clap(long, env = "MINT_DERIVATION_PATH")]
    pub derivation_path: Option<String>,
    #[clap(flatten)]
    pub info: MintInfoConfig,
    #[clap(flatten)]
    pub lightning_fee: LightningFeeConfig,
    #[clap(flatten)]
    pub server: ServerConfig,
    #[clap(flatten)]
    pub database: DatabaseConfig,

    #[clap(long, env = "MINT_LIGHTNING_BACKEND")]
    pub lightning_backend: LightningTypeVariant,

    #[clap(long, env = "MINT_BTC_ONCHAIN_BACKEND")]
    pub btconchain_backend: Option<BtcOnchainTypeVariant>,

    #[clap(flatten)]
    pub tracing: Option<TracingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightningTypeVariant {
    Lnbits,
    Alby,
    Strike,
    Lnd,
    Cln,
}

impl FromStr for LightningTypeVariant {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Lnbits" => Ok(Self::Lnbits),
            "Alby" => Ok(Self::Alby),
            "Strike" => Ok(Self::Strike),
            "Lnd" => Ok(Self::Lnd),
            "Cln" => Ok(Self::Cln),
            _ => Err("no match"),
        }
    }
}

#[derive(Debug, Clone, Default, Parser)]
pub struct TracingConfig {
    #[clap(long, env = "MINT_TRACING_ENDPOINT")]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MintConfig {
    pub privatekey: String,
    pub derivation_path: Option<String>,
    pub info: MintInfoConfig,
    pub lightning_fee: LightningFeeConfig,
    pub server: ServerConfig,
    pub btconchain_backend: Option<BtcOnchainConfig>,
    pub lightning_backend: Option<LightningType>,
    pub tracing: Option<TracingConfig>,
    pub database: DatabaseConfig,
}

impl From<(Opts, LightningType, Option<BtcOnchainConfig>)> for MintConfig {
    fn from((opts, ln, btc): (Opts, LightningType, Option<BtcOnchainConfig>)) -> Self {
        Self {
            privatekey: opts.privatekey,
            derivation_path: opts.derivation_path,
            info: opts.info,
            lightning_fee: opts.lightning_fee,
            server: opts.server,
            btconchain_backend: btc,
            lightning_backend: Some(ln),
            tracing: opts.tracing,
            database: opts.database,
        }
    }
}

impl MintConfig {
    pub fn read_config_with_defaults() -> Self {
        let opts: Opts = Opts::parse();

        let lightning = match opts.lightning_backend {
            LightningTypeVariant::Lnd => LightningType::Lnd(LndLightningSettings::parse()),
            LightningTypeVariant::Lnbits => LightningType::Lnbits(LnbitsLightningSettings::parse()),
            LightningTypeVariant::Strike => LightningType::Strike(StrikeLightningSettings::parse()),
            LightningTypeVariant::Alby => LightningType::Alby(AlbyLightningSettings::parse()),
            LightningTypeVariant::Cln => LightningType::Cln(ClnLightningSettings::parse()),
        };

        let btc_onchain: Option<BtcOnchainConfig> = match opts.btconchain_backend {
            Some(BtcOnchainTypeVariant::Lnd) => {
                let cfg = BtcOnchainConfig::parse();
                Some(BtcOnchainConfig {
                    onchain_type: Some(BtcOnchainType::Lnd(LndLightningSettings::parse())),
                    ..cfg
                })
            }
            None => None,
        };

        (opts, lightning, btc_onchain).into()
    }
}

impl MintConfig {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        private_key: String,
        derivation_path: Option<String>,
        info: MintInfoConfig,
        lightning_fee: LightningFeeConfig,
        server: ServerConfig,
        database: DatabaseConfig,
        btconchain_backend: Option<BtcOnchainConfig>,
        lightning_backend: Option<LightningType>,
        tracing: Option<TracingConfig>,
    ) -> Self {
        Self {
            privatekey: private_key,
            derivation_path,
            info,
            lightning_fee,
            server,
            btconchain_backend,
            lightning_backend,
            tracing,
            database,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub struct BtcOnchainConfig {
    #[clap(skip)]
    pub onchain_type: Option<BtcOnchainType>,

    #[clap(
        long,
        default_value_t = 1,
        env = "MINT_BTC_ONCHAIN_BACKEND_MIN_CONFIRMATIONS"
    )]
    pub min_confirmations: u8,

    #[clap(
        long,
        default_value_t = 10_000,
        env = "MINT_BTC_ONCHAIN_BACKEND_MIN_AMOUNT"
    )]
    pub min_amount: u64,

    #[clap(
        long,
        default_value_t = 1_000_000,
        env = "MINT_BTC_ONCHAIN_BACKEND_MAX_AMOUNT"
    )]
    pub max_amount: u64,
}

impl Default for BtcOnchainConfig {
    fn default() -> Self {
        Self {
            onchain_type: None,
            min_confirmations: 1,
            min_amount: 10_000,
            max_amount: 1_000_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtcOnchainType {
    Lnd(LndLightningSettings),
}

#[derive(Debug, Clone)]
pub enum BtcOnchainTypeVariant {
    Lnd,
}

impl FromStr for BtcOnchainTypeVariant {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Lnd" => Ok(Self::Lnd),
            _ => Err("no match"),
        }
    }
}

impl From<BtcOnchainConfig> for Nut18 {
    fn from(settings: BtcOnchainConfig) -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfigBtcOnchainMint {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: settings.min_amount,
                max_amount: settings.max_amount,
                min_confirmations: settings.min_confirmations,
            }],
        }
    }
}

impl From<BtcOnchainConfig> for Nut19 {
    fn from(settings: BtcOnchainConfig) -> Self {
        Self {
            supported: true,
            payment_methods: vec![PaymentMethodConfigBtcOnchainMelt {
                payment_method: PaymentMethod::BtcOnchain,
                unit: CurrencyUnit::Sat,
                min_amount: settings.min_amount,
                max_amount: settings.max_amount,
            }],
        }
    }
}

#[derive(Debug, Clone, Parser)]
pub struct DatabaseConfig {
    #[clap(long, env = "MINT_DB_URL")]
    pub db_url: String,

    #[clap(long, default_value_t = 5, env = "MINT_DB_MAX_CONNECTIONS")]
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_url: "".to_owned(),
            max_connections: 5,
        }
    }
}

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    #[clap(long, default_value = "[::]:3338", env = "MINT_HOST_PORT")]
    pub host_port: SocketAddr,
    #[clap(long, env = "MINT_SERVE_WALLET_PATH")]
    pub serve_wallet_path: Option<PathBuf>,
    #[clap(long, env = "MINT_API_PREFIX")]
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
#[derive(Deserialize, Serialize, Debug, Clone, Default, Parser)]
pub struct MintInfoConfig {
    #[clap(long, default_value = "moksha-mint", env = "MINT_INFO_NAME")]
    pub name: Option<String>,

    #[clap(long, default_value_t = true, env = "MINT_INFO_VERSION")]
    pub version: bool,

    #[clap(long, env = "MINT_INFO_DESCRIPTION")]
    pub description: Option<String>,

    #[clap(long, env = "MINT_INFO_DESCRIPTION_LONG")]
    pub description_long: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_EMAIL")]
    pub contact_email: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_TWITTER")]
    pub contact_twitter: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_NOSTR")]
    pub contact_nostr: Option<String>,

    #[clap(long, env = "MINT_INFO_MOTD")]
    pub motd: Option<String>,
    // FIXME add missing fields for v1/info endpoint nut4/nut5 payment_methods, nut4 disabled flag
}

impl From<MintInfoConfig> for Vec<ContactInfoResponse> {
    fn from(info: MintInfoConfig) -> Vec<ContactInfoResponse> {
        [
            info.contact_email.map(ContactInfoResponse::email),
            info.contact_twitter.map(ContactInfoResponse::twitter),
            info.contact_nostr.map(ContactInfoResponse::nostr),
        ]
        .iter()
        .filter_map(|contact| contact.to_owned())
        .collect()
    }
}
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct BuildParams {
    pub commit_hash: Option<String>,
    pub build_time: Option<String>,
    pub cargo_pkg_version: Option<String>,
}

impl BuildParams {
    pub fn from_env() -> Self {
        Self {
            commit_hash: env::var("COMMITHASH").ok(),
            build_time: env::var("BUILDTIME").ok(),
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

#[derive(Debug, Clone, Parser)]
pub struct LightningFeeConfig {
    #[clap(long, default_value_t = 1.0, env = "MINT_LIGHTNING_FEE_PERCENT")]
    pub fee_percent: f32,
    #[clap(long, default_value_t = 4_000, env = "MINT_LIGHTNING_FEE_RESERVE_MIN")]
    pub fee_reserve_min: u64,
    // TODO check if fee_percent is in range
}

impl LightningFeeConfig {
    pub const fn new(fee_percent: f32, fee_reserve_min: u64) -> Self {
        Self {
            fee_percent,
            fee_reserve_min,
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

impl Default for LightningFeeConfig {
    fn default() -> Self {
        Self {
            fee_percent: 1.0,
            fee_reserve_min: 4000,
        }
    }
}
