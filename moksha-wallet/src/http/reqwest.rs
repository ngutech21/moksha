use super::CrossPlatformHttpClient;
use crate::error::MokshaWalletError;
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Response, StatusCode,
};
use serde_json::Value;
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
                        // FIXME cleanup code
                        let data: Value = serde_json::from_str(&response_text)
                            .map_err(|_| MokshaWalletError::UnexpectedResponse(response_text))
                            .expect("invalid value");
                        let detail = data["detail"].as_str().expect("detail not found");
                        // let data = serde_json::from_str::<CashuErrorResponse>(&response_text)
                        //     .map_err(|_| MokshaWalletError::UnexpectedResponse(response_text))
                        //     .unwrap();

                        // FIXME: use the error code to return a proper error
                        match detail {
                            "Lightning invoice not paid yet." => {
                                Err(MokshaWalletError::InvoiceNotPaidYet(0, detail.to_owned()))
                            }
                            _ => Err(MokshaWalletError::MintError(detail.to_owned())),
                        }
                    }
                }
            }
            _ => {
                let response_text = response.text().await?;
                let data: Value = serde_json::from_str(&response_text)
                    .map_err(|_| MokshaWalletError::UnexpectedResponse(response_text))
                    .expect("invalid value");
                let detail = data["detail"].as_str().expect("detail not found");

                // FIXME: use the error code to return a proper error
                match detail {
                    "Lightning invoice not paid yet." => {
                        Err(MokshaWalletError::InvoiceNotPaidYet(0, detail.to_owned()))
                    }
                    _ => Err(MokshaWalletError::MintError(detail.to_owned())),
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
