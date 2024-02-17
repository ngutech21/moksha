use crate::error::MokshaMintError;
use crate::routes::btconchain::{
    get_melt_btconchain, get_melt_quote_btconchain, get_mint_quote_btconchain,
    post_melt_btconchain, post_melt_quote_btconchain, post_mint_btconchain,
    post_mint_quote_btconchain,
};
use crate::routes::default::{
    get_info, get_keys, get_keys_by_id, get_keysets, get_melt_quote_bolt11, get_mint_quote_bolt11,
    post_melt_bolt11, post_melt_quote_bolt11, post_mint_bolt11, post_mint_quote_bolt11, post_swap,
};
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{get_service, post};
use axum::{middleware, Router};
use axum::{routing::get, Json};

use moksha_core::keyset::{V1Keyset, V1Keysets};
use moksha_core::proof::Proofs;
use moksha_core::proof::{P2SHScript, Proof};
use tracing_subscriber::EnvFilter;
use utoipa_swagger_ui::SwaggerUi;

use crate::mint::Mint;

use moksha_core::blind::BlindedMessage;
use moksha_core::blind::BlindedSignature;
use moksha_core::primitives::{
    CurrencyUnit, GetMeltOnchainResponse, KeyResponse, KeysResponse, MintInfoResponse,
    MintLegacyInfoResponse, Nut10, Nut11, Nut12, Nut14, Nut15, Nut4, Nut5, Nut7, Nut8, Nut9, Nuts,
    PaymentMethod, PostMeltBolt11Request, PostMeltBolt11Response, PostMeltQuoteBolt11Request,
    PostMeltQuoteBolt11Response, PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse,
    PostMintBolt11Request, PostMintBolt11Response, PostMintQuoteBolt11Request,
    PostMintQuoteBolt11Response, PostMintQuoteOnchainRequest, PostMintQuoteOnchainResponse,
    PostSwapRequest, PostSwapResponse,
};

use tower_http::services::ServeDir;

use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use utoipa::OpenApi;

use crate::routes::legacy::{
    get_legacy_keys, get_legacy_keysets, get_legacy_mint, post_legacy_check_fees, post_legacy_melt,
    post_legacy_mint, post_legacy_split,
};

pub async fn run_server(mint: Mint) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    if let Some(ref buildtime) = mint.build_params.build_time {
        info!("build time: {}", buildtime);
    }
    if let Some(ref commithash) = mint.build_params.commit_hash {
        info!("git commit-hash: {}", commithash);
    }
    if let Some(ref serve_wallet_path) = mint.config.server.serve_wallet_path {
        info!("serving wallet from path: {:?}", serve_wallet_path);
    }
    info!("listening on: {}", &mint.config.server.host_port);
    info!("mint-info: {:?}", mint.config.info);
    info!("lightning fee-reserve: {:?}", mint.config.lightning_fee);
    info!("lightning-backend: {}", mint.lightning_type);

    if let Some(ref onchain) = mint.config.btconchain_backend {
        info!("onchain-type: {:?}", onchain.onchain_type);
        info!(
            "btconchain-min-confirmations: {}",
            onchain.min_confirmations
        );
        info!("btconchain-min-amount: {}", onchain.min_amount);
        info!("btconchain-max-amount: {}", onchain.max_amount);
    } else {
        info!("btconchain-backend is not configured");
    }

    let listener = tokio::net::TcpListener::bind(&mint.config.server.host_port)
        .await
        .unwrap();

    axum::serve(
        listener,
        app(mint)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_headers(Any)
                    .allow_methods(Any)
                    .expose_headers(Any),
            )
            .into_make_service(),
    )
    .await?;

    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::default::get_keys,
        crate::routes::default::get_keys_by_id,
        crate::routes::default::get_keysets,
        crate::routes::default::post_mint_bolt11,
        crate::routes::default::post_mint_quote_bolt11,
        crate::routes::default::get_mint_quote_bolt11,
        crate::routes::default::post_melt_bolt11,
        crate::routes::default::post_melt_quote_bolt11,
        crate::routes::default::get_melt_quote_bolt11,
        crate::routes::default::post_swap,
        crate::routes::default::get_info,
        get_health,
        crate::routes::btconchain::post_mint_quote_btconchain,
        crate::routes::btconchain::get_mint_quote_btconchain,
        crate::routes::btconchain::post_mint_btconchain,
        crate::routes::btconchain::post_melt_quote_btconchain,
        crate::routes::btconchain::get_melt_quote_btconchain,
        crate::routes::btconchain::post_melt_btconchain,
        crate::routes::btconchain::get_melt_btconchain
    ),
    components(schemas(
        MintInfoResponse,
        Nuts,
        Nut4,
        Nut5,
        Nut7,
        Nut8,
        Nut9,
        Nut10,
        Nut11,
        Nut12,
        CurrencyUnit,
        PaymentMethod,
        KeysResponse,
        KeyResponse,
        V1Keysets,
        V1Keyset,
        BlindedMessage,
        BlindedSignature,
        Proof,
        Proofs,
        PostMintQuoteBolt11Request,
        PostMintQuoteBolt11Response,
        PostMeltQuoteBolt11Request,
        PostMeltQuoteBolt11Response,
        PostMeltBolt11Request,
        PostMeltBolt11Response,
        PostMintBolt11Request,
        PostMintBolt11Response,
        PostSwapRequest,
        PostSwapResponse,
        P2SHScript,
        Nut14,
        Nut15,
        PostMintQuoteOnchainRequest,
        PostMintQuoteOnchainResponse,
        PostMeltQuoteOnchainRequest,
        PostMeltQuoteOnchainResponse,
        GetMeltOnchainResponse
    ))
)]
struct ApiDoc;

fn app(mint: Mint) -> Router {
    let legacy_routes = Router::new()
        .route("/keys", get(get_legacy_keys))
        .route("/keysets", get(get_legacy_keysets))
        .route("/mint", get(get_legacy_mint).post(post_legacy_mint))
        .route("/checkfees", post(post_legacy_check_fees))
        .route("/melt", post(post_legacy_melt))
        .route("/split", post(post_legacy_split))
        .route("/info", get(get_legacy_info));

    let default_routes = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/keys", get(get_keys))
        .route("/v1/keys/:id", get(get_keys_by_id))
        .route("/v1/keysets", get(get_keysets))
        .route("/v1/mint/quote/bolt11", post(post_mint_quote_bolt11))
        .route("/v1/mint/quote/bolt11/:quote", get(get_mint_quote_bolt11))
        .route("/v1/mint/bolt11", post(post_mint_bolt11))
        .route("/v1/melt/quote/bolt11", post(post_melt_quote_bolt11))
        .route("/v1/melt/quote/bolt11/:quote", get(get_melt_quote_bolt11))
        .route("/v1/melt/bolt11", post(post_melt_bolt11))
        .route("/v1/swap", post(post_swap))
        .route("/v1/info", get(get_info));

    let btconchain_routes = if mint.onchain.is_some() {
        Router::new()
            .route(
                "/v1/mint/quote/btconchain",
                post(post_mint_quote_btconchain),
            )
            .route(
                "/v1/mint/quote/btconchain/:quote",
                get(get_mint_quote_btconchain),
            )
            .route("/v1/mint/btconchain", post(post_mint_btconchain))
            .route(
                "/v1/melt/quote/btconchain",
                post(post_melt_quote_btconchain),
            )
            .route(
                "/v1/melt/quote/btconchain/:quote",
                get(get_melt_quote_btconchain),
            )
            .route("/v1/melt/btconchain", post(post_melt_btconchain))
            .route("/v1/melt/btconchain/:txid", get(get_melt_btconchain))
    } else {
        Router::new()
    };

    let general_routes = Router::new().route("/health", get(get_health));

    let server_config = mint.config.server.clone();
    let prefix = server_config.api_prefix.unwrap_or_else(|| "".to_owned());

    let router = Router::new()
        .nest(&prefix, legacy_routes)
        .nest(&prefix, default_routes)
        .nest(&prefix, btconchain_routes)
        .nest("", general_routes)
        .with_state(mint)
        .layer(TraceLayer::new_for_http());

    if let Some(ref serve_wallet_path) = server_config.serve_wallet_path {
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

async fn get_legacy_info(
    State(mint): State<Mint>,
) -> Result<Json<MintLegacyInfoResponse>, MokshaMintError> {
    let mint_info = MintLegacyInfoResponse {
        name: mint.config.info.name,
        pubkey: mint.keyset_legacy.mint_pubkey,
        version: match mint.config.info.version {
            true => Some(mint.build_params.full_version()),
            _ => None,
        },
        description: mint.config.info.description,
        description_long: mint.config.info.description_long,
        contact: None, // FIXME set contact
        nuts: vec![
            "NUT-00".to_string(),
            "NUT-01".to_string(),
            "NUT-02".to_string(),
            "NUT-03".to_string(),
            "NUT-04".to_string(),
            "NUT-05".to_string(),
            "NUT-06".to_string(),
            "NUT-08".to_string(),
            "NUT-09".to_string(),
        ],
        motd: mint.config.info.motd,
        parameter: Default::default(),
    };
    Ok(Json(mint_info))
}

#[utoipa::path(
        get,
        path = "/health",
        responses(
            (status = 200, description = "health check")
        ),
    )]
async fn get_health() -> impl IntoResponse {
    StatusCode::OK
}

// ######################################################################################################

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use crate::{btconchain::MockBtcOnchain, config::MintConfig, server::app};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use moksha_core::{
        keyset::{Keysets, V1Keysets},
        primitives::{CurrencyUnit, KeysResponse, MintLegacyInfoResponse},
    };
    use secp256k1::PublicKey;
    use tower::ServiceExt;

    use crate::{
        config::MintInfoConfig,
        database::MockDatabase,
        lightning::{LightningType, MockLightning},
        mint::Mint,
    };

    #[tokio::test]
    async fn test_get_keys() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
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
        let app = app(create_mock_mint(Default::default()));
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
        let mint_info_settings = MintInfoConfig {
            name: Some("Bob's Cashu mint".to_string()),
            version: true,
            description: Some("A mint for testing".to_string()),
            description_long: Some("A mint for testing long".to_string()),
            ..Default::default()
        };
        let app = app(create_mock_mint(mint_info_settings));
        let response = app
            .oneshot(Request::builder().uri("/info").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let info = serde_json::from_slice::<MintLegacyInfoResponse>(&body)?;
        assert!(!info.parameter.peg_out_only);
        assert_eq!(info.nuts.len(), 9);
        assert_eq!(info.name, Some("Bob's Cashu mint".to_string()));
        assert_eq!(info.description, Some("A mint for testing".to_string()));
        assert_eq!(
            info.description_long,
            Some("A mint for testing long".to_string())
        );
        Ok(())
    }

    fn create_mock_mint(info: MintInfoConfig) -> Mint {
        let db = Arc::new(MockDatabase::new());
        let lightning = Arc::new(MockLightning::new());

        Mint::new(
            lightning,
            LightningType::Lnbits(Default::default()),
            db,
            MintConfig {
                info,
                privatekey: "mytestsecret".to_string(),
                ..Default::default()
            },
            Default::default(),
            Some(Arc::new(MockBtcOnchain::default())),
        )
    }

    // ################ v1 api tests #####################

    #[tokio::test]
    async fn test_get_keys_v1() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(Request::builder().uri("/v1/keys").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keys: KeysResponse = serde_json::from_slice(&body)?;
        let keysets = keys.keysets;
        assert_eq!(&1, &keysets.len());
        assert_eq!(64, keysets[0].keys.len());
        assert_eq!(16, keysets[0].id.len());
        assert_eq!(CurrencyUnit::Sat, keysets[0].unit);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_keysets_v1() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(Request::builder().uri("/v1/keysets").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keysets = serde_json::from_slice::<V1Keysets>(&body)?;
        assert_eq!(1, keysets.keysets.len());
        assert_eq!(16, keysets.keysets[0].id.len());
        Ok(())
    }

    // ### v1 api tests

    #[tokio::test]
    async fn test_get_v1_keys() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(Request::builder().uri("/v1/keys").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keys: KeysResponse = serde_json::from_slice(&body)?;
        assert_eq!(1, keys.keysets.len());
        assert_eq!(
            64,
            keys.keysets.first().expect("keyset not found").keys.len()
        );
        println!("{:#?}", keys.keysets.first().unwrap().id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_v1_keys_id_invalid() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/keys/unknownkeyset")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_v1_keys_id() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/keys/00f545318e4fad2b")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keys: KeysResponse = serde_json::from_slice(&body)?;
        assert_eq!(1, keys.keysets.len());
        assert_eq!(
            64,
            keys.keysets.first().expect("keyset not found").keys.len()
        );
        assert_eq!(
            "00f545318e4fad2b",
            keys.keysets.first().expect("keyset not found").id
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_get_v1_keysets() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(Request::builder().uri("/v1/keysets").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let keys: V1Keysets = serde_json::from_slice(&body)?;
        assert_eq!(1, keys.keysets.len());
        let keyset = keys.keysets.first().expect("keyset not found");
        assert!(keyset.active);
        assert_eq!(CurrencyUnit::Sat, keyset.unit);
        assert_eq!("00f545318e4fad2b", keyset.id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_health() -> anyhow::Result<()> {
        let app = app(create_mock_mint(Default::default()));
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }
}
