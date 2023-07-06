use moksha_fedimint::FedimintWallet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let wallet = FedimintWallet::new().await?;
    //FedimintWallet::connect("connect-string").await?;
    wallet.balance().await?;
    wallet.mint(100).await?;
    Ok(())
}
