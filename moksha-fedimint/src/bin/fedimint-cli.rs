use moksha_fedimint::FedimintWallet;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workdir = workdir()?;
    // FedimintWallet::connect(workdir.clone(), "fed115da59spv337s2ce5ms460520ctyqprjtc28vxdlwrlq95tdmrjmxprewc3keddw9cg0ek9xxp0zrcqq4waen5te0xyerwt3s9cczuvf6xyurzde59am68e394t2t4m60dzkjxacz7m5").await?;
    let wallet = FedimintWallet::new(workdir.clone()).await?;
    if FedimintWallet::is_initialized(&workdir) {
        println!("Wallet is initialized");
        let balance = wallet.balance().await?;
        println!("Balance: {}", balance);
    } else {
        println!("Wallet is not initialized");
    }

    // let (operation_id, invoice) = wallet.get_mint_payment_request(1_000).await?;
    // println!("Invoice: \n{}", invoice);
    // wallet.mint(operation_id, 1_000).await?;
    Ok(())
}

fn workdir() -> anyhow::Result<std::path::PathBuf> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    PathBuf::from_str(&format!("{base_dir}/data")).map_err(|e| anyhow::anyhow!(e))
}
