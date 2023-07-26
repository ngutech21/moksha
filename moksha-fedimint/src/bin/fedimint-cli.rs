#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let connection = std::env::var("CLI_FEDIMINT_CONNECTION")?;
    let workdir = workdir()?;
    if !moksha_fedimint::FedimintWallet::is_initialized(&workdir) {
        moksha_fedimint::FedimintWallet::connect(workdir.clone(), &connection).await?;
    }

    let wallet = moksha_fedimint::FedimintWallet::new(workdir.clone()).await?;
    println!("Wallet is initialized");
    let balance = wallet.balance().await?;
    println!("Balance: {}", balance);

    let (operation_id, invoice) = wallet.get_mint_payment_request(1_000).await?;
    println!("Invoice: \n{}", invoice);
    wallet.mint(operation_id).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn workdir() -> anyhow::Result<std::path::PathBuf> {
    use std::str::FromStr;
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    std::path::PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
}
