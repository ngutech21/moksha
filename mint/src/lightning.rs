use lnbits_rust::{
    api::invoice::{CreateInvoiceParams, CreateInvoiceResult, DecodedInvoice, PayInvoiceResult},
    LNBitsClient,
};

use crate::error::CashuMintError;

#[derive(Clone)]
pub struct Lightning {
    pub client: LNBitsClient,
}

impl Lightning {
    pub fn new(
        wallet_id: String,
        admin_key: String,
        invoice_read_key: String,
        url: String,
    ) -> Self {
        Self {
            client: LNBitsClient::new(&wallet_id, &admin_key, &invoice_read_key, &url, None)
                .expect("Can not create Lnbits client"),
        }
    }

    pub async fn create_invoice(&self, amount: i64) -> CreateInvoiceResult {
        self.client
            .create_invoice(&CreateInvoiceParams {
                amount,
                unit: "sat".to_string(),
                memo: None,
                expiry: Some(10000),
                webhook: None,
                internal: None,
            })
            .await
            .unwrap()
    }

    pub async fn pay_invoice(
        &self,
        payment_request: String,
    ) -> Result<PayInvoiceResult, CashuMintError> {
        Ok(self.client.pay_invoice(&payment_request).await?)
    }

    pub async fn decode_invoice(
        &self,
        payment_request: String,
    ) -> Result<DecodedInvoice, CashuMintError> {
        // TODO use lightning_invoice from LDK instead of calling the API
        Ok(self.client.decode_invoice(&payment_request).await?)
    }
}
