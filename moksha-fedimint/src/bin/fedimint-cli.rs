use moksha_fedimint::FedimintWallet;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let connection = env::var("CLI_FEDIMINT_CONNECTION")?;
    let workdir = workdir()?;
    if !FedimintWallet::is_initialized(&workdir) {
        FedimintWallet::connect(workdir.clone(), &connection).await?;
    }

    let wallet = FedimintWallet::new(workdir.clone()).await?;
    println!("Wallet is initialized");
    let balance = wallet.balance().await?;
    println!("Balance: {}", balance);

    let (operation_id, invoice) = wallet.get_mint_payment_request(1_000).await?;
    println!("Invoice: \n{}", invoice);
    wallet.mint(operation_id).await?;
    Ok(())
}

fn workdir() -> anyhow::Result<std::path::PathBuf> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
}
