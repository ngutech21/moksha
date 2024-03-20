use crate::{bitcoin_client::BitcoinClient, lnd_client::LndClient};
use mokshamint::{
    config::{BtcOnchainConfig, DatabaseConfig, ServerConfig},
    lightning::LightningType,
    mint::MintBuilder,
};

pub async fn fund_mint_lnd(amount: u64) -> anyhow::Result<()> {
    let btc_client = BitcoinClient::new_local()?;
    btc_client.mine_blocks(108)?;
    let lnd_client = LndClient::new_mint_lnd().await?;
    let lnd_address = lnd_client.new_address().await?;
    btc_client.send_to_address(
        &lnd_address,
        bitcoincore_rpc::bitcoin::Amount::from_sat(amount),
    )?;
    std::thread::sleep(std::time::Duration::from_millis(3_000));
    Ok(())
}

pub async fn open_channel_with_wallet(amount: u64) -> anyhow::Result<()> {
    let wallet_lnd = LndClient::new_wallet_lnd().await?;
    let wallet_pubkey = wallet_lnd.get_pubkey().await?;

    let mint_lnd = LndClient::new_mint_lnd().await?;
    mint_lnd
        .connect_to_peer(&wallet_pubkey, "lnd-wallet:9735")
        .await?;
    let mine_blocks = mint_lnd.open_channel(&wallet_pubkey, amount).await?;
    if mine_blocks {
        let btc_client = BitcoinClient::new_local()?;
        btc_client.mine_blocks(3)?;
    }
    Ok(())
}

pub async fn start_mint(
    host_port: u16,
    ln: LightningType,
    btc_onchain: Option<BtcOnchainConfig>,
) -> anyhow::Result<()> {
    let db_config = DatabaseConfig {
        db_url: format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            host_port
        ),
        ..Default::default()
    };

    let mint = MintBuilder::new()
        .with_private_key("my_private_key".to_string())
        .with_server(Some(ServerConfig {
            host_port: "127.0.0.1:8686".parse()?,
            ..Default::default()
        }))
        .with_db(Some(db_config))
        .with_lightning(ln)
        .with_btc_onchain(btc_onchain)
        .with_fee(Some((0.0, 0).into()))
        .build();

    mokshamint::server::run_server(mint.await.expect("Can not connect to lightning backend"))
        .await?;
    Ok(())
}

pub fn read_fixture(name: &str) -> anyhow::Result<String> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let raw_token = std::fs::read_to_string(format!("{base_dir}/tests/fixtures/{name}"))?;
    Ok(raw_token.trim().to_string())
}
