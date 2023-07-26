pub mod client;
pub mod error;

pub mod localstore;
pub mod wallet;

pub mod btcprice;

#[cfg(not(target_arch = "wasm32"))]
pub mod reqwest_client;

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlx_localstore;
