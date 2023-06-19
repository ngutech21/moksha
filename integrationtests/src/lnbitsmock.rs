use axum::{response::IntoResponse, routing::get, Router};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use lightning::ln::PaymentSecret;
use lightning_invoice::{Currency, InvoiceBuilder};
use secp256k1::Secp256k1;
use secp256k1::SecretKey;

#[derive(Debug, Deserialize, Serialize)]
struct CreateInvoiceRequest {
    out: bool,
    amount: Option<u64>,
    bolt11: Option<String>,
    memo: Option<String>,
    expiry: Option<i32>,
    unit: Option<String>,
    webhook: Option<String>,
    internal: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CreateInvoiceResponse {
    payment_hash: String,
    payment_request: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct PaymentStatus {
    paid: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PayInvoiceResponse {
    payment_hash: String,
}

async fn post_invoice(
    create_invoice: axum::Json<CreateInvoiceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if !create_invoice.out {
        let private_key = SecretKey::from_slice(
            &[
                0xe1, 0x26, 0xf6, 0x8f, 0x7e, 0xaf, 0xcc, 0x8b, 0x74, 0xf5, 0x4d, 0x26, 0x9f, 0xe2,
                0x06, 0xbe, 0x71, 0x50, 0x00, 0xf9, 0x4d, 0xac, 0x06, 0x7d, 0x1c, 0x04, 0xa8, 0xca,
                0x3b, 0x2d, 0xb7, 0x34,
            ][..],
        )
        .unwrap();

        let payment_hash = sha256::Hash::from_slice(&[0; 32][..]).unwrap();
        let payment_secret = PaymentSecret([42u8; 32]);

        let invoice = InvoiceBuilder::new(Currency::Regtest)
            .description(create_invoice.memo.clone().unwrap_or("".to_string()))
            .amount_milli_satoshis(create_invoice.amount.unwrap())
            .payment_hash(payment_hash)
            .payment_secret(payment_secret)
            .current_timestamp()
            .min_final_cltv_expiry_delta(144)
            .build_signed(|hash| Secp256k1::new().sign_ecdsa_recoverable(hash, &private_key))
            .unwrap();

        let payment_hash = invoice.payment_hash().to_string();
        let payment_request = invoice.to_string();
        let response = CreateInvoiceResponse {
            payment_hash,
            payment_request,
        };
        Ok(serde_json::to_string(&response).unwrap())
    } else {
        let payment_hash = "1234567890abcdef".to_string();
        let response = PayInvoiceResponse { payment_hash };
        Ok(serde_json::to_string(&response).unwrap())
    }
}

async fn get_payment(
    _payment_hash: axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(axum::Json(PaymentStatus { paid: true }))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/v1/payments", get(get_payment).post(post_invoice));
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
