use axum::extract::State;
use axum::Json;
use axum::{response::IntoResponse, routing::get, routing::post, Router};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use lightning::ln::PaymentSecret;
use lightning_invoice::{Currency, InvoiceBuilder};
use secp256k1::Secp256k1;
use secp256k1::SecretKey;

#[derive(Default, Debug, Deserialize, Serialize)]
struct CreateInvoiceRequest {
    out: bool,
    amount: Option<u64>,
    bolt11: Option<String>,
    memo: Option<String>,
    expiry: Option<u32>,
    unit: Option<String>,
    webhook: Option<String>,
    internal: Option<bool>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
struct CreateInvoiceResponse {
    payment_hash: String,
    payment_request: Option<String>,
    checking_id: Option<String>,
    lnurl_response: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PaymentStatus {
    paid: bool,
}

async fn post_invoice(
    State(private_key): State<SecretKey>,
    params: axum::Json<CreateInvoiceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if !params.out {
        let payment_hash = sha256::Hash::from_slice(&[0; 32][..]).expect("Can't create hash");
        let payment_secret = PaymentSecret([42u8; 32]);

        let invoice = InvoiceBuilder::new(Currency::Regtest)
            .description(params.memo.clone().unwrap_or("".to_string()))
            .amount_milli_satoshis(params.amount.expect("amount is not set"))
            .payment_hash(payment_hash)
            .payment_secret(payment_secret)
            .current_timestamp()
            .min_final_cltv_expiry_delta(144)
            .build_signed(|hash| Secp256k1::new().sign_ecdsa_recoverable(hash, &private_key))
            .expect("Can't create invoice");

        let payment_hash = invoice.payment_hash().to_string();
        let payment_request = invoice.to_string();
        let response = CreateInvoiceResponse {
            payment_hash,
            payment_request: Some(payment_request),
            checking_id: Some(
                "caf224bb1dc543a3da2783c431d096428b7ea35807361a92868bdd0ac6de0f22".to_owned(),
            ),
            ..Default::default()
        };
        Ok(Json(response))
    } else {
        let payment_hash = "1234567890abcdef".to_string();
        let response = CreateInvoiceResponse {
            payment_hash,
            ..Default::default()
        };
        Ok(Json(response))
    }
}

async fn get_payment(
    _payment_hash: axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json(PaymentStatus { paid: true }))
}

// TODO upgrade to axum 0.7.3
pub async fn run_server(port: u16) -> anyhow::Result<()> {
    let private_key = SecretKey::new(&mut rand::thread_rng());
    let app = Router::new()
        .route("/api/v1/payments/:payment_hash", get(get_payment))
        .route("/api/v1/payments", post(post_invoice))
        .with_state(private_key);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
