use lnbits_rust::{
    api::invoice::{CreateInvoiceParams, CreateInvoiceResult, PayInvoiceResult},
    LNBitsClient,
};

use crate::error::CashuMintError;

use lightning_invoice::Invoice as LNInvoice;
use std::str::FromStr;

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

    pub async fn is_invoice_paid(&self, invoice: String) -> Result<bool, CashuMintError> {
        let decoded_invoice = self.decode_invoice(invoice).await.unwrap();
        Ok(self
            .client
            .is_invoice_paid(&decoded_invoice.payment_hash().to_string())
            .await
            .unwrap())
    }

    pub async fn create_invoice(&self, amount: u64) -> CreateInvoiceResult {
        let amount: i64 = amount.try_into().unwrap(); // FIXME use u64
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
        self.client
            .pay_invoice(&payment_request)
            .await
            .map_err(|err| CashuMintError::PayInvoice(payment_request, err))
    }

    pub async fn decode_invoice(
        &self,
        payment_request: String,
    ) -> Result<LNInvoice, CashuMintError> {
        // TODO use lightning_invoice from LDK instead of calling the API
        // self.client
        //     .decode_invoice(&payment_request)
        //     .await
        //     .map_err(|err| CashuMintError::DecodeInvoice(payment_request, err))
        LNInvoice::from_str(&payment_request)
            .map_err(|err| CashuMintError::DecodeInvoice(payment_request, err))
    }
}
