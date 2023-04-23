use std::collections::HashMap;

use axum::Router;
use axum::{routing::get, Json};
use cashurs_core::model::MintKeyset;
use hyper::Method;
use secp256k1::PublicKey;
use serde_json::Error;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{event, Level};

use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    event!(Level::INFO, "startup");

    let addr = "[::]:3338".parse()?;
    event!(Level::INFO, "listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET]),
                )
                .into_make_service(),
        )
        .await?;

    Ok(())
}

fn app() -> Router {
    Router::new()
        .route("/keys", get(get_keys))
        .route("/keysets", get(get_keysets))
        .layer(TraceLayer::new_for_http())
}

//{
//  "1": "03a40f20667ed53513075dc51e715ff2046cad64eb68960632269ba7f0210e38bc",//
//  "2": "03fd4ce5a16b65576145949e6f99f445f8249fee17c606b688b504a849cdc452de",
//  "4": "02648eccfa4c026960966276fa5a4cae46ce0fd432211a4f449bf84f13aa5f8303",
//  "8": "02fdfd6796bfeac490cbee12f778f867f0a2c68f6508d17c649759ea0dc3547528",

async fn get_keys() -> Result<Json<HashMap<u64, PublicKey>>, ()> {
    let keys = MintKeyset::new("foo".to_string());
    Ok(Json(keys.public_keys))
}

async fn get_keysets() -> &'static str {
    "get keysets"
}
