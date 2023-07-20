use mokshamint::{info::MintInfoSettings, MintBuilder};
use std::env;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let mint_info_settings = envy::prefixed("MINT_INFO_")
        .from_env::<MintInfoSettings>()
        .expect("Please provide mint info");

    let mint = MintBuilder::new()
        .with_mint_info(mint_info_settings)
        .with_private_key(get_env("MINT_PRIVATE_KEY"))
        .with_db(get_env("MINT_DB_PATH"))
        .with_lnbits(get_env("LNBITS_URL"), get_env("LNBITS_ADMIN_KEY"))
        .with_fee(
            get_env("LIGHTNING_FEE_PERCENT").parse()?,
            get_env("LIGHTNING_RESERVE_FEE_MIN").parse()?,
        )
        .build();

    mokshamint::run_server(mint, 3338).await
}

fn get_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} not found", key))
}
