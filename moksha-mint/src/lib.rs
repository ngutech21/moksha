use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::post;
use axum::Router;
use axum::{routing::get, Json};
use error::MokshaMintError;
use hyper::Method;
use mint::{LightningFeeConfig, Mint};
use model::{GetMintQuery, PostMintQuery};
use moksha_core::model::{
    CheckFeesRequest, CheckFeesResponse, Keysets, PaymentRequest, PostMeltRequest,
    PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
};
use secp256k1::PublicKey;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{event, Level};

use crate::lightning::LnbitsLightning;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod database;
mod error;
mod lightning;
mod lnbits;
pub mod mint;
mod model;

#[derive(Debug, Default)]
pub struct MintBuilder {
    private_key: Option<String>,
    lnbits_admin_key: Option<String>,
    lnbits_url: Option<String>,
    db_path: Option<String>,
    fee_percent: Option<f32>,
    fee_reserve_min: Option<u64>,
}

impl MintBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_private_key(mut self, private_key: String) -> MintBuilder {
        self.private_key = Some(private_key);
        self
    }

    pub fn with_db(mut self, db_path: String) -> MintBuilder {
        self.db_path = Some(db_path);
        self
    }

    pub fn with_lnbits(mut self, url: String, admin_key: String) -> MintBuilder {
        self.lnbits_admin_key = Some(admin_key);
        self.lnbits_url = Some(url);
        self
    }

    pub fn with_fee(mut self, fee_percent: f32, fee_reserve_min: u64) -> MintBuilder {
        self.fee_percent = Some(fee_percent);
        self.fee_reserve_min = Some(fee_reserve_min);
        self
    }

    pub fn build(self) -> Mint {
        let ln = Arc::new(LnbitsLightning::new(
            self.lnbits_admin_key.expect("LNBITS_ADMIN_KEY not set"),
            self.lnbits_url.expect("LNBITS_URL not set"),
        ));

        let db = Arc::new(database::RocksDB::new(
            self.db_path.expect("MINT_DB_PATH not set"),
        ));

        let fee_config = LightningFeeConfig::new(
            self.fee_percent.expect("LIGHTNING_FEE_PERCENT not set"),
            self.fee_reserve_min
                .expect("LIGHTNING_RESERVE_FEE_MIN not set"),
        );

        Mint::new(
            self.private_key.expect("MINT_PRIVATE_KEY not set"),
            "".to_string(),
            ln,
            db,
            fee_config,
        )
    }
}

pub async fn run_server(mint: Mint, port: u16) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    event!(Level::INFO, "startup");

    let addr = format!("[::]:{port}").parse()?;
    event!(Level::INFO, "listening on {}", addr);

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
    State(mint): State<Mint>,
    Json(split_request): Json<PostSplitRequest>,
) -> Result<Json<PostSplitResponse>, MokshaMintError> {
    let (fst, snd) = mint
        .split(
            split_request.amount,
            &split_request.proofs,
            &split_request.outputs,
        )
        .await?;

    Ok(Json(PostSplitResponse { fst, snd }))
}

async fn post_melt(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltRequest>,
) -> Result<Json<PostMeltResponse>, MokshaMintError> {
    let (paid, preimage, change) = mint
        .melt(melt_request.pr, &melt_request.proofs, &melt_request.outputs)
        .await?;

    Ok(Json(PostMeltResponse {
        paid,
        preimage,
        change,
    }))
}

async fn post_check_fees(
    State(mint): State<Mint>,
    Json(_check_fees): Json<CheckFeesRequest>,
) -> Result<Json<CheckFeesResponse>, MokshaMintError> {
    let invoice = mint.lightning.decode_invoice(_check_fees.pr).await?;

    Ok(Json(CheckFeesResponse {
        fee: mint.fee_reserve(
            invoice
                .amount_milli_satoshis()
                .ok_or_else(|| error::MokshaMintError::InvalidAmount)?,
        ),
    }))
}

async fn get_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<GetMintQuery>,
) -> Result<Json<PaymentRequest>, MokshaMintError> {
    let (pr, hash) = mint.create_invoice(mint_query.amount).await?;
    Ok(Json(PaymentRequest { pr, hash }))
}

async fn post_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<PostMintQuery>,
    Json(blinded_messages): Json<PostMintRequest>,
) -> Result<Json<PostMintResponse>, MokshaMintError> {
    event!(
        Level::INFO,
        "post_mint: {mint_query:#?} {blinded_messages:#?}"
    );

    let promises = mint
        .mint_tokens(mint_query.hash, &blinded_messages.outputs)
        .await?;
    Ok(Json(PostMintResponse { promises }))
}

async fn get_keys(
    State(mint): State<Mint>,
) -> Result<Json<HashMap<u64, PublicKey>>, MokshaMintError> {
    Ok(Json(mint.keyset.public_keys))
}

async fn get_keysets(State(mint): State<Mint>) -> Result<Json<Keysets>, MokshaMintError> {
    Ok(Json(Keysets::new(vec![mint.keyset.keyset_id])))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use hyper::{Body, Request, StatusCode};
    use moksha_core::model::Keysets;
    use secp256k1::PublicKey;
    use tower::ServiceExt;

    use crate::{
        app,
        database::MockDatabase,
        lightning::MockLightning,
        mint::{LightningFeeConfig, Mint},
    };

    #[tokio::test]
    async fn test_get_keys() -> anyhow::Result<()> {
        let app = app(create_mock_mint());
        let response = app
            .oneshot(Request::builder().uri("/keys").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await?;
        let keys: HashMap<u64, PublicKey> = serde_json::from_slice(&body)?;
        assert_eq!(64, keys.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_keysets() -> anyhow::Result<()> {
        let app = app(create_mock_mint());
        let response = app
            .oneshot(Request::builder().uri("/keysets").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await?;
        let keysets = serde_json::from_slice::<Keysets>(&body)?;
        assert_eq!(Keysets::new(vec!["53eJP2+qJyTd".to_string()]), keysets);
        Ok(())
    }

    fn create_mock_mint() -> Mint {
        let db = Arc::new(MockDatabase::new());
        let lightning = Arc::new(MockLightning::new());
        Mint::new(
            "mytestsecret".to_string(),
            "".to_string(),
            lightning,
            db,
            LightningFeeConfig::default(),
        )
    }
}
