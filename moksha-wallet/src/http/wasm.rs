use moksha_core::primitives::CashuErrorResponse;

use crate::error::MokshaWalletError;
use url::Url;

use super::CrossPlatformHttpClient;
use gloo_net::http::{Request, Response};

impl CrossPlatformHttpClient {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn do_get<T: serde::de::DeserializeOwned>(
        &self,
        url: &Url,
    ) -> Result<T, MokshaWalletError> {
        let resp = Request::get(url.as_str()).send().await?;
        Self::extract_response_data::<T>(resp).await
    }

    pub async fn do_post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &Url,
        body: &B,
    ) -> Result<T, MokshaWalletError> {
        let resp = Request::post(url.as_str())
            .header("content-type", "application/json")
            .json(body)?
            .send()
            .await?;
        Self::extract_response_data::<T>(resp).await
    }

    pub async fn get_status(&self, url: &Url) -> Result<u16, MokshaWalletError> {
        let resp = Request::get(url.as_str()).send().await?;

        Ok(resp.status())
    }

    async fn extract_response_data<T: serde::de::DeserializeOwned>(
        response: Response,
    ) -> Result<T, MokshaWalletError> {
        match response.status() {
            200 => {
                let response_text = response.text().await.unwrap(); // FIXME handle error
                match serde_json::from_str::<T>(&response_text) {
                    Ok(data) => Ok(data),
                    Err(_) => {
                        let data = serde_json::from_str::<CashuErrorResponse>(&response_text)
                            .map_err(|_| MokshaWalletError::UnexpectedResponse(response_text))
                            .unwrap();

                        // FIXME: use the error code to return a proper error
                        match data.detail.as_str() {
                            "Lightning invoice not paid yet." => {
                                Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.detail))
                            }
                            _ => Err(MokshaWalletError::MintError(data.detail)),
                        }
                    }
                }
            }
            _ => {
                let txt = response.text().await.unwrap(); // FIXME handle error
                let data = serde_json::from_str::<CashuErrorResponse>(&txt)
                    .map_err(|_| MokshaWalletError::UnexpectedResponse(txt))
                    .unwrap();

                // FIXME: use the error code to return a proper error
                match data.detail.as_str() {
                    "Lightning invoice not paid yet." => {
                        Err(MokshaWalletError::InvoiceNotPaidYet(data.code, data.detail))
                    }
                    _ => Err(MokshaWalletError::MintError(data.detail)),
                }
            }
        }
    }
}
