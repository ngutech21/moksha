use crate::error::MokshaWalletError;

async fn execute_request(url: &str) -> Result<String, MokshaWalletError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let request_client = reqwest::Client::new();
        let response = request_client.get(url).send().await?;
        Ok(response.text().await?)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let resp = gloo_net::http::Request::get(url).send().await.unwrap();
        Ok(resp.text().await?)
    }
}

pub async fn get_btcprice() -> Result<f64, MokshaWalletError> {
    let response = execute_request(
        "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd&precision=18",
    )
    .await?;

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
        assert!(btc_price > 0.0);
        let price_per_sat = btc_price / 100_000_000.0;
        println!("price_per_sat: {}", price_per_sat);
        println!("btcprice: {}", btc_price);
        println!("5000 sats {}", price_per_sat * 5000.0);
    }
}
