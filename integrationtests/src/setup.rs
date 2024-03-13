use crate::{
    bitcoin_client::BitcoinClient,
    lnd_client::{self, LndClient},
};
use mokshamint::{
    config::{BtcOnchainConfig, BtcOnchainType, DatabaseConfig, ServerConfig},
    lightning::{lnd::LndLightningSettings, LightningType},
    mint::MintBuilder,
};

pub async fn fund_lnd(amount: u64) -> anyhow::Result<()> {
    let btc_client = BitcoinClient::new_local()?;
    btc_client.mine_blocks(101)?;
    let lnd_client = LndClient::new_local().await?;
    let lnd_address = lnd_client.new_address().await?;
    btc_client.send_to_address(
        &lnd_address,
        bitcoincore_rpc::bitcoin::Amount::from_sat(amount),
    )?;
    Ok(())
}

pub async fn start_mint(host_port: u16) -> anyhow::Result<()> {
    let db_config = DatabaseConfig {
        db_url: format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            host_port
        ),
        ..Default::default()
    };

    // FIXME clean up
    let lnd_settings = LndLightningSettings::new(
        lnd_client::LND_ADDRESS.parse().expect("invalid url"),
        "../data/lnd1/tls.cert".into(),
        "../data/lnd1/data/chain/bitcoin/regtest/admin.macaroon".into(),
    );

    let mint = MintBuilder::new()
        .with_private_key("my_private_key".to_string())
        .with_server(Some(ServerConfig {
            host_port: "127.0.0.1:8686".parse()?,
            ..Default::default()
        }))
        .with_db(Some(db_config))
        .with_lightning(LightningType::Lnd(lnd_settings.clone()))
        .with_btc_onchain(Some(BtcOnchainConfig {
            onchain_type: Some(BtcOnchainType::Lnd(lnd_settings)),
            ..Default::default()
        }))
        .with_fee(Some((0.0, 0).into()))
        .build();

    mokshamint::server::run_server(mint.await.expect("Can not connect to lightning backend"))
        .await?;
    Ok(())
}
