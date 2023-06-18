#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    cashursmint::run_server(3338).await
}
