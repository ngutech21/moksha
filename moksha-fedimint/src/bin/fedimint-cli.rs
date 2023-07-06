use moksha_fedimint::FedimintWallet;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let workdir = workdir()?;
    let wallet = FedimintWallet::new(workdir).await?;
    //FedimintWallet::connect(workdir, "connect-string").await?;
    wallet.balance().await?;
    wallet.mint(100).await?;
    Ok(())
}

fn workdir() -> anyhow::Result<std::path::PathBuf> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
}
