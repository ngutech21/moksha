use std::time::Duration;

use assert_cmd::Command;
use itests::{
    lnd_client,
    setup::{fund_mint_lnd, start_mint},
};
use mokshamint::lightning::{lnd::LndLightningSettings, LightningType};

use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_cli() -> anyhow::Result<()> {
    let node = Postgres::default()
        .with_host_auth()
        .with_tag("16.6-alpine")
        .start()
        .await?;
    let host_port = node.get_host_port_ipv4(5432).await?;

    fund_mint_lnd(2_000_000).await?;

    // start mint server
    tokio::spawn(async move {
        let lnd_settings = LndLightningSettings::new(
            lnd_client::LND_MINT_ADDRESS.parse().expect("invalid url"),
            "../data/lnd-mint/tls.cert".into(),
            "../data/lnd-mint/data/chain/bitcoin/regtest/admin.macaroon".into(),
        );

        let ln_type = LightningType::Lnd(lnd_settings.clone());

        start_mint(host_port, ln_type, None)
            .await
            .expect("Could not start mint server");
    });

    // Wait for the server to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    // compile the moksha-cli binary and run it
    let mut cmd = Command::cargo_bin("moksha-cli")?;
    cmd.arg("info");
    let output = cmd.unwrap();
    assert!(output.status.success());
    Ok(())
}
