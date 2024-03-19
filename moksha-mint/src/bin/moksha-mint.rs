use mokshamint::{
    config::{MintConfig, TracingConfig},
    mint::MintBuilder,
};
use std::env;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Sampler;

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
        btconchain_backend,
        lightning_backend,
        tracing,
        database,
    } = MintConfig::read_config_with_defaults();

    init_tracing(tracing.clone())?;

    let mint = MintBuilder::new()
        .with_mint_info(Some(info))
        .with_server(Some(server))
        .with_private_key(privatekey)
        .with_derivation_path(derivation_path)
        .with_db(Some(database))
        .with_lightning(lightning_backend.expect("lightning not set"))
        .with_btc_onchain(btconchain_backend)
        .with_fee(Some(lightning_fee))
        .with_tracing(tracing)
        .build()
        .await;

    mokshamint::server::run_server(mint?).await
}

fn init_tracing(tr: Option<TracingConfig>) -> anyhow::Result<()> {
    let otlp_tracer = if tr.is_some() {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter().http().with_endpoint(
                    tr.unwrap_or_default()
                        .endpoint
                        .expect("No endpoint for tracing found"),
                ),
            )
            .with_trace_config(
                opentelemetry_sdk::trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_resource(opentelemetry_sdk::Resource::new(vec![KeyValue::new(
                        "service.name",
                        "moksha-mint",
                    )])),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .with(otlp_tracer)
        .try_init()?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Prod,
}

impl core::fmt::Display for AppEnv {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Prod => write!(f, "prod"),
        }
    }
}
