#[cfg(not(target_os = "espidf"))]
#[cfg(not(target_arch = "wasm32"))]
pub mod reqwest;

#[cfg(any(target_arch = "wasm32", target_os = "espidf"))]
pub mod wasm;

#[derive(Debug, Clone)]
pub struct CrossPlatformHttpClient {
    #[cfg(not(target_os = "espidf"))]
    #[cfg(not(target_arch = "wasm32"))]
    client: ::reqwest::Client,
}

impl Default for CrossPlatformHttpClient {
    fn default() -> Self {
        Self::new()
    }
}
