use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::routing::post;
use axum::Router;
use axum::{routing::get, Json};
use cashurs_core::dhke;
use cashurs_core::model::{
    BlindedSignature, CheckFeesRequest, CheckFeesResponse, Keysets, PaymentRequest,
    PostMeltRequest, PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest,
    PostSplitResponse,
};
use dotenvy::dotenv;
use hyper::Method;
use mint::Mint;
use model::MintQuery;
use secp256k1::PublicKey;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{event, Level};

use crate::lightning::Lightning;
use std::env;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod lightning;
mod mint;
mod model;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    event!(Level::INFO, "startup");

    let addr = "[::]:3338".parse()?;
    event!(Level::INFO, "listening on {}", addr);

    dotenv().expect(".env file not found");
    let mint = create_mint();

    axum::Server::bind(&addr)
        .serve(
            app(mint)
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST]),
                )
                .into_make_service(),
        )
        .await?;

    Ok(())
}

fn create_mint() -> Mint {
    let ln = Lightning::new(
        env::var("LNBITS_WALLET_ID").expect("LNBITS_WALLET_ID not found"),
        env::var("LNBITS_ADMIN_KEY").expect("LNBITS_ADMIN_KEY not found"),
        env::var("LNBITS_INVOICE_READ_KEY").expect("LNBITS_INVOICE_READ_KEY not found"),
        env::var("LNBITS_URL").expect("LNBITS_URL not found"),
    );
    Mint::new(
        env::var("MINT_PRIVATE_KEY").expect("MINT_PRIVATE_KEY not found"),
        ln,
    )
}

fn app(mint: Mint) -> Router {
    Router::new()
        .route("/keys", get(get_keys))
        .route("/keysets", get(get_keysets))
        .route("/mint", get(get_mint).post(post_mint))
        .route("/checkfees", post(post_check_fees))
        .route("/melt", post(post_melt))
        .route("/split", post(post_split))
        .with_state(mint)
        .layer(TraceLayer::new_for_http())
}

async fn post_split(
    Json(_check_fees): Json<PostSplitRequest>,
) -> Result<Json<PostSplitResponse>, ()> {
    Ok(Json(PostSplitResponse {
        fst: vec![],
        snd: vec![],
    }))
}

async fn post_melt(
    State(_mint): State<Mint>,
    Json(_check_fees): Json<PostMeltRequest>,
) -> Result<Json<PostMeltResponse>, ()> {
    Ok(Json(PostMeltResponse {
        paid: true,
        preimage: "dummy preimage".to_string(), // FIXME connect to lightning
        change: vec![],
    }))
}

async fn post_check_fees(
    Json(_check_fees): Json<CheckFeesRequest>,
) -> Result<Json<CheckFeesResponse>, ()> {
    Ok(Json(CheckFeesResponse { fee: 1 }))
}

async fn get_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<MintQuery>,
) -> Result<Json<PaymentRequest>, ()> {
    println!("amount: {mint_query:#?}",);
    // FIXME return error of amount is None
    let amount: i64 = mint_query.amount.unwrap().try_into().unwrap(); // FIXME use u64
    let invoice = mint.lightning.create_invoice(amount).await;
    //let pr = "lnbc2500u1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdq5xysxxatsyp3k7enxv4jsxqzpuaztrnwngzn3kdzw5hydlzf03qdgm2hdq27cqv3agm2awhz5se903vruatfhq77w3ls4evs3ch9zw97j25emudupq63nyw24cg27h2rspfj9srp";
    Ok(Json(PaymentRequest {
        pr: invoice.payment_request,
        hash: invoice.payment_hash,
    }))
}

async fn post_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<MintQuery>,
    Json(blinded_messages): Json<PostMintRequest>,
) -> Result<Json<PostMintResponse>, ()> {
    event!(
        Level::INFO,
        "post_mint: {mint_query:#?} {blinded_messages:#?}"
    );

    let promises = blinded_messages
        .outputs
        .iter()
        .map(|blinded_msg| {
            let private_key = mint.keyset.private_keys.get(&blinded_msg.amount).unwrap(); // FIXME unwrap
            let blinded_sig = dhke::step2_bob(blinded_messages.outputs[0].b_, private_key).unwrap();
            BlindedSignature {
                id: Some(mint.keyset.keyset_id.clone()),
                amount: blinded_msg.amount,
                c_: blinded_sig,
            }
        })
        .collect::<Vec<BlindedSignature>>();
    Ok(Json(PostMintResponse { promises }))
}

async fn get_keys(State(mint): State<Mint>) -> Result<Json<HashMap<u64, PublicKey>>, ()> {
    Ok(Json(mint.keyset.public_keys))
}

async fn get_keysets(State(mint): State<Mint>) -> Result<Json<Keysets>, ()> {
    Ok(Json(Keysets {
        keysets: vec![mint.keyset.keyset_id],
    }))
}
