use moksha_fedimint::FedimintWallet;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workdir = workdir()?;
    FedimintWallet::connect(workdir.clone(), "fed115zsqx0frxykrhf00294tms3qtt4tsuvrmn2q5hyyk7yj62gjygq2tytc63lrxly5mljd35k8udyexqq5waen5te0xyerwt3s9cczuvf68qcnwdp0lrcks9alpj0legwfx02szhs6nf").await?;
    let wallet = FedimintWallet::new(workdir).await?;
    let balance = wallet.balance().await?;
    println!("Balance: {}", balance);
    wallet.mint(1_000).await?;
    Ok(())
}

fn workdir() -> anyhow::Result<std::path::PathBuf> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
}
