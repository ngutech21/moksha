use cashursmint::MintBuilder;
use std::env;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let mint = MintBuilder::new()
        .with_private_key(get_env("MINT_PRIVATE_KEY"))
        .with_db(get_env("MINT_DB_PATH"))
        .with_lnbits(get_env("LNBITS_URL"), get_env("LNBITS_ADMIN_KEY"))
        .with_fee(
            get_env("LIGHTNING_FEE_PERCENT").parse()?,
            get_env("LIGHTNING_RESERVE_FEE_MIN").parse()?,
        )
        .build();

    cashursmint::run_server(mint, 3338).await
}

fn get_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} not found", key))
}
