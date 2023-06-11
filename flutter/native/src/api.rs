// This is the entry point of your Rust library.
// When adding new code to your project, note that only items used
// here will be transformed to their Dart equivalents.

use std::io::Error;

use tokio::runtime::Runtime;

pub fn say_hello() -> String {
    "Hello from Rust!".to_string()
}

pub fn generate_qrcode(amount: u8) -> anyhow::Result<String> {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let result = internal_generate_qrcode(amount).await;
        result.map_err(anyhow::Error::from)
    })
}

async fn internal_generate_qrcode(amount: u8) -> Result<String, Error> {
    Ok(format!("qr code for value {amount}"))
}
