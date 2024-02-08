use super::CrossPlatformHttpClient;
use crate::error::MokshaWalletError;
use moksha_core::primitives::CashuErrorResponse;
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Response, StatusCode,
};
use url::Url;

impl CrossPlatformHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn extract_response_data<T: serde::de::DeserializeOwned>(
        response: Response,
    ) -> Result<T, MokshaWalletError> {
        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await?;
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
                let txt = response.text().await?;
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

    pub async fn do_get<T: serde::de::DeserializeOwned>(
        &self,
        url: &Url,
    ) -> Result<T, MokshaWalletError> {
        let resp = self.client.get(url.clone()).send().await?;
        Self::extract_response_data::<T>(resp).await
    }

    pub async fn do_post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &Url,
        body: &B,
    ) -> Result<T, MokshaWalletError> {
        let resp = self
            .client
            .post(url.clone())
            .header(CONTENT_TYPE, HeaderValue::from_str("application/json")?)
            .body(serde_json::to_string(body)?)
            .send()
            .await?;
        Self::extract_response_data::<T>(resp).await
    }

    pub async fn get_status(&self, url: &Url) -> Result<u16, MokshaWalletError> {
        let resp = self.client.get(url.to_owned()).send().await?;
        Ok(resp.status().as_u16())
    }
}
