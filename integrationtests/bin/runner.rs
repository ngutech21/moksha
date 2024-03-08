use std::time::Instant;

use bitcoincore_rpc::bitcoin::Amount;
use itests::{bitcoin_client::BitcoinClient, lnd_client::LndClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let btc = BitcoinClient::new_local()?;
    let start = Instant::now();
    btc.mine_blocks(101)?;
    let duration = start.elapsed();
    println!("Time elapsed in mine_blocks() is: {:?}", duration);

    let lnd = LndClient::new_local().await?;
    let lnd_address = lnd.new_address().await?;
    println!("lnd address: {}", lnd_address);

    btc.send_to_address(&lnd_address, Amount::from_sat(210_234))?;
    let balance_lnd = lnd.get_balance(&lnd_address, 0).await?;
    println!("balance lnd: {:?}", balance_lnd);
    Ok(())
}
