use std::env;

use serde_derive::{Deserialize, Serialize};

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

#[derive(Clone, Debug)]
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
