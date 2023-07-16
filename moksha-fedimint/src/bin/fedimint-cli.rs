use moksha_fedimint::FedimintWallet;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workdir = workdir()?;
    FedimintWallet::connect(workdir.clone(), "fed11sasnnkm0d2exauq87za95te0lshg54mzdd8d039c7ezaf0de0rlvncl8w3sljwde03vcylzyhxurjqqewaen5te0xc6zuv3jxuhrzvpk9ccnxve68qcnwdp03vlw9pz6kfe2p3r2easqhrrjkv").await?;
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
