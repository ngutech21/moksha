use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::error::MokshaMintError;
use axum::extract::{Query, Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{get_service, post};
use axum::{middleware, Router};
use axum::{routing::get, Json};
use moksha_core::keyset::{Keysets, V1Keysets};

use crate::mint::Mint;
use crate::model::{GetMintQuery, PostMintQuery, Quote};
use moksha_core::primitives::{
    CheckFeesRequest, CheckFeesResponse, CurrencyUnit, KeyResponse, KeysResponse, MintInfoResponse,
    PaymentRequest, PostMeltBolt11Request, PostMeltBolt11Response, PostMeltQuoteBolt11Request,
    PostMeltQuoteBolt11Response, PostMeltRequest, PostMeltResponse, PostMintBolt11Request,
    PostMintBolt11Response, PostMintQuoteBolt11Request, PostMintQuoteBolt11Response,
    PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
};
use secp256k1::PublicKey;

use tower_http::services::ServeDir;

use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{event, info, Level};

use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn run_server(
    mint: Mint,
    addr: SocketAddr,
    serve_wallet_path: Option<PathBuf>,
    api_prefix: Option<String>,
) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("listening on: {}", addr);
    info!("mint_info: {:?}", mint.mint_info);
    info!("lightning_backend: {}", mint.lightning_type);
    if serve_wallet_path.is_some() {
        info!(
            "serving wallet from path: {:?}",
            serve_wallet_path.clone().unwrap()
        );
    }

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(
        listener,
        app(mint, serve_wallet_path, api_prefix)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_headers(Any)
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST]),
            )
            .into_make_service(),
    )
    .await?;

    Ok(())
}

fn app(mint: Mint, serve_wallet_path: Option<PathBuf>, prefix: Option<String>) -> Router {
    let legacy_routes = Router::new()
        .route("/keys", get(get_legacy_keys))
        .route("/keysets", get(get_legacy_keysets))
        .route("/mint", get(get_legacy_mint).post(post_legacy_mint))
        .route("/checkfees", post(post_legacy_check_fees))
        .route("/melt", post(post_legacy_melt))
        .route("/split", post(post_split))
        .route("/info", get(get_legacy_info));

    let routes = Router::new()
        .route("/v1/keys", get(get_keys))
        .route("/v1/keysets", get(get_keysets))
        .route("/v1/mint/quote/bolt11", post(post_mint_quote_bolt11))
        .route("/v1/mint/bolt11", post(post_mint_bolt11))
        .route("/v1/melt/quote/bolt11", post(post_melt_quote_bolt11))
        .route("/v1/melt/bolt11", post(post_melt_bolt11))
        .route("/v1/swap", post(post_split))
        .route("/v1/info", get(get_legacy_info));

    let prefix = prefix.unwrap_or_else(|| "".to_owned());

    let router = Router::new()
        .nest(&prefix, legacy_routes)
        .nest(&prefix, routes)
        .with_state(mint)
        .layer(TraceLayer::new_for_http());

    if let Some(serve_wallet_path) = serve_wallet_path {
        return router.nest_service(
            "/",
            get_service(ServeDir::new(serve_wallet_path))
                .layer(middleware::from_fn(add_response_headers)),
        );
    }
    router
}

/// This function adds response headers that are specific to Flutter web applications.
///
/// It sets the `cross-origin-embedder-policy` header to `require-corp` and the
/// `cross-origin-opener-policy` header to `same-origin`. These headers are necessary
/// for some features of Flutter web applications, such as isolating the application
/// from potential security threats in other browsing contexts.
///
/// # Arguments
///
/// * `req` - The incoming request.
/// * `next` - The next middleware or endpoint in the processing chain.
///
/// # Returns
///
/// This function returns a `Result` with the modified response, or an error if
/// something went wrong while processing the request or response.
async fn add_response_headers(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut res = next.run(req).await;

    res.headers_mut().insert(
        HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    );
    res.headers_mut().insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    Ok(res)
}

async fn post_split(
    State(mint): State<Mint>,
    Json(split_request): Json<PostSplitRequest>,
) -> Result<Json<PostSplitResponse>, MokshaMintError> {
    let response = mint
        .split(&split_request.proofs, &split_request.outputs)
        .await?;

    Ok(Json(response))
}

async fn post_legacy_melt(
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

async fn post_legacy_check_fees(
    State(mint): State<Mint>,
    Json(_check_fees): Json<CheckFeesRequest>,
) -> Result<Json<CheckFeesResponse>, MokshaMintError> {
    let invoice = mint.lightning.decode_invoice(_check_fees.pr).await?;

    Ok(Json(CheckFeesResponse {
        fee: mint.fee_reserve(
            invoice
                .amount_milli_satoshis()
                .ok_or_else(|| crate::error::MokshaMintError::InvalidAmount)?,
        ),
    }))
}

async fn get_legacy_info(
    State(mint): State<Mint>,
) -> Result<Json<MintInfoResponse>, MokshaMintError> {
    let mint_info = MintInfoResponse {
        name: mint.mint_info.name,
        pubkey: mint.keyset.mint_pubkey,
        version: match mint.mint_info.version {
            true => Some(env!("CARGO_PKG_VERSION").to_owned()),
            _ => None,
        },
        description: mint.mint_info.description,
        description_long: mint.mint_info.description_long,
        contact: mint.mint_info.contact,
        nuts: vec![
            "NUT-00".to_string(),
            "NUT-01".to_string(),
            "NUT-02".to_string(),
            "NUT-03".to_string(),
            "NUT-04".to_string(),
            "NUT-05".to_string(),
            "NUT-06".to_string(),
            "NUT-09".to_string(),
        ],
        motd: mint.mint_info.motd,
        parameter: Default::default(),
    };
    Ok(Json(mint_info))
}

async fn get_legacy_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<GetMintQuery>,
) -> Result<Json<PaymentRequest>, MokshaMintError> {
    let (pr, hash) = mint.create_invoice(mint_query.amount).await?;
    Ok(Json(PaymentRequest { pr, hash }))
}

async fn post_legacy_mint(
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

async fn get_legacy_keys(
    State(mint): State<Mint>,
) -> Result<Json<HashMap<u64, PublicKey>>, MokshaMintError> {
    Ok(Json(mint.keyset.public_keys))
}

async fn get_legacy_keysets(State(mint): State<Mint>) -> Result<Json<Keysets>, MokshaMintError> {
    Ok(Json(Keysets::new(vec![mint.keyset.keyset_id])))
}

// ######################################################################################################

async fn get_keys(State(mint): State<Mint>) -> Result<Json<KeysResponse>, MokshaMintError> {
    Ok(Json(KeysResponse {
        keysets: vec![KeyResponse {
            id: mint.keyset.keyset_id.clone(),
            unit: CurrencyUnit::Sat,
            keys: mint.keyset.public_keys.clone(),
        }],
    }))
}

async fn get_keysets(State(mint): State<Mint>) -> Result<Json<V1Keysets>, MokshaMintError> {
    Ok(Json(V1Keysets::new(
        mint.keyset.keyset_id,
        CurrencyUnit::Sat,
        true,
    )))
}

async fn post_mint_quote_bolt11(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteBolt11Request>,
) -> Result<Json<PostMintQuoteBolt11Response>, MokshaMintError> {
    // FIXME check currency unit
    let (pr, _hash) = mint.create_invoice(request.amount).await?;

    let invoice = mint.lightning.decode_invoice(pr.clone()).await?;

    let quote = Quote::new(pr.clone());
    let quote_id = quote.quote_id.to_string();
    mint.db.add_quote(quote_id.clone(), quote)?;

    Ok(Json(PostMintQuoteBolt11Response {
        quote: quote_id,
        request: pr,
        paid: false,
        expiry: invoice.expiry_time().as_secs(), // FIXME check if this is correct
    }))
}

async fn post_mint_bolt11(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBolt11Request>,
) -> Result<Json<PostMintBolt11Response>, MokshaMintError> {
    let quotes = &mint.db.get_quotes()?;
    let quote = quotes
        .get(request.quote.as_str())
        .ok_or_else(|| crate::error::MokshaMintError::InvalidQuote(request.quote.clone()))?;

    let signatures = mint
        .mint_tokens(quote.payment_request.clone(), &request.outputs)
        .await?;
    Ok(Json(PostMintBolt11Response { signatures }))
}

async fn post_melt_quote_bolt11(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltQuoteBolt11Request>,
) -> Result<Json<PostMeltQuoteBolt11Response>, MokshaMintError> {
    let invoice = mint
        .lightning
        .decode_invoice(melt_request.request.clone())
        .await?;
    let amount = invoice
        .amount_milli_satoshis()
        .ok_or_else(|| crate::error::MokshaMintError::InvalidAmount)?;
    let fee_reserve = mint.fee_reserve(amount);

    Ok(Json(PostMeltQuoteBolt11Response {
        amount,
        fee_reserve,
        quote: melt_request.request.clone(), // FIXME use uuid as quote
    }))
}

async fn post_melt_bolt11(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltBolt11Request>,
) -> Result<Json<PostMeltBolt11Response>, MokshaMintError> {
    let quote = mint.db.get_quote(melt_request.quote)?;

    let (paid, preimage, _change) = mint
        .melt(quote.payment_request, &melt_request.inputs, &[])
        .await?;

    Ok(Json(PostMeltBolt11Response {
        paid,
        payment_preimage: preimage,
        change: vec![], // FIXME return change
    }))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use crate::server::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use moksha_core::{keyset::Keysets, primitives::MintInfoResponse};
    use secp256k1::PublicKey;
    use tower::ServiceExt;

    use crate::{
        database::MockDatabase,
        info::MintInfoSettings,
        lightning::{LightningType, MockLightning},
        mint::{LightningFeeConfig, Mint},
    };

    #[tokio::test]
    async fn test_get_keys() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()), None, None);
        let response = app
            .oneshot(Request::builder().uri("/keys").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keys: HashMap<u64, PublicKey> = serde_json::from_slice(&body)?;
        assert_eq!(64, keys.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_keysets() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()), None, None);
        let response = app
            .oneshot(Request::builder().uri("/keysets").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keysets = serde_json::from_slice::<Keysets>(&body)?;
        assert_eq!(Keysets::new(vec!["53eJP2+qJyTd".to_string()]), keysets);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_info() -> anyhow::Result<()> {
        let mint_info_settings = MintInfoSettings {
            name: Some("Bob's Cashu mint".to_string()),
            version: true,
            description: Some("A mint for testing".to_string()),
            description_long: Some("A mint for testing long".to_string()),
            ..Default::default()
        };
        let app = app(create_mock_mint(mint_info_settings), None, None);
        let response = app
            .oneshot(Request::builder().uri("/info").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let info = serde_json::from_slice::<MintInfoResponse>(&body)?;
        assert!(!info.parameter.peg_out_only);
        assert_eq!(info.nuts.len(), 8);
        assert_eq!(info.name, Some("Bob's Cashu mint".to_string()));
        assert_eq!(info.description, Some("A mint for testing".to_string()));
        assert_eq!(
            info.description_long,
            Some("A mint for testing long".to_string())
        );
        Ok(())
    }

    fn create_mock_mint(mint_info: MintInfoSettings) -> Mint {
        let db = Arc::new(MockDatabase::new());
        let lightning = Arc::new(MockLightning::new());

        Mint::new(
            "mytestsecret".to_string(),
            "".to_string(),
            lightning,
            LightningType::Lnbits(Default::default()),
            db,
            LightningFeeConfig::default(),
            mint_info,
        )
    }
}