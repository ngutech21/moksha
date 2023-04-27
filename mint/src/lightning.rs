use lnbits_rust::{
    api::invoice::{CreateInvoiceParams, CreateInvoiceResult},
    LNBitsClient,
};

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
            client: LNBitsClient::new(
                &wallet_id,        // wallet-id
                &admin_key,        // admin-key
                &invoice_read_key, // invoice-read-key
                &url,              // url
                None,
            )
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
}
