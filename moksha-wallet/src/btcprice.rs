use crate::error::MokshaWalletError;

pub async fn get_btcprice() -> Result<f64, MokshaWalletError> {
    let request_client = reqwest::Client::new();
    let url =
        "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd&precision=18";
    let response = request_client.get(url).send().await?;
    let response = response.text().await?;
    let response: serde_json::Value = serde_json::from_str(&response)?;
    let btcprice = response["bitcoin"]["usd"].as_f64().expect("No btcprice");
    Ok(btcprice)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get_btcprice() {
        let btc_price = get_btcprice().await.unwrap();
        let price_per_sat = btc_price / 100_000_000.0;
        println!("price_per_sat: {}", price_per_sat);
        println!("btcprice: {}", btc_price);
        println!("5000 sats {}", price_per_sat * 5000.0);
    }
}
