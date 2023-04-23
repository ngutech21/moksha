use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::Router;
use axum::{routing::get, Json};
use cashurs_core::model::MintKeyset;
use hyper::Method;
use secp256k1::PublicKey;
use serde::Serialize;
use serde_json::Error;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{event, Level};

use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

//type LocalKeyset = HashMap<u64, PublicKey>;

//type DBState = State<Arc<Box<dyn DB + Send + Sync>>>;

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
    let keyset = MintKeyset::new("mysecret".to_string());
    Router::new()
        .route("/keys", get(get_keys))
        .route("/keysets", get(get_keysets))
        .with_state(keyset)
        .layer(TraceLayer::new_for_http())
}

async fn get_keys(State(keyset): State<MintKeyset>) -> Result<Json<HashMap<u64, PublicKey>>, ()> {
    Ok(Json(keyset.public_keys))
}

#[derive(Clone, Debug, Serialize)]
struct Keyset {
    keysets: Vec<String>,
}

async fn get_keysets(State(keyset): State<MintKeyset>) -> Result<Json<Keyset>, ()> {
    Ok(Json(Keyset {
        keysets: vec![keyset.keyset_id],
    }))
}
