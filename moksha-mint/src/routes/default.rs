use axum::{
    extract::{Path, State},
    Json,
};
use moksha_core::{
    keyset::V1Keysets,
    primitives::{
        Bolt11MeltQuote, Bolt11MintQuote, CurrencyUnit, KeyResponse, KeysResponse,
        MintInfoResponse, Nuts, PaymentMethod, PostMeltBolt11Request, PostMeltBolt11Response,
        PostMeltQuoteBolt11Request, PostMeltQuoteBolt11Response, PostMintBolt11Request,
        PostMintBolt11Response, PostMintQuoteBolt11Request, PostMintQuoteBolt11Response,
        PostSwapRequest, PostSwapResponse,
    },
};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{
    config::{BtcOnchainConfig, MintConfig},
    error::MokshaMintError,
    mint::Mint,
};
use chrono::{Duration, Utc};
use std::str::FromStr;

#[utoipa::path(
        post,
        path = "/v1/swap",
        request_body = PostSwapRequest,
        responses(
            (status = 200, description = "post swap", body = [PostSwapResponse])
        ),
    )]
#[instrument(name = "post_swap", skip(mint), err)]
pub async fn post_swap(
    State(mint): State<Mint>,
    Json(swap_request): Json<PostSwapRequest>,
) -> Result<Json<PostSwapResponse>, MokshaMintError> {
    let response = mint
        .swap(&swap_request.inputs, &swap_request.outputs, &mint.keyset)
        .await?;

    Ok(Json(PostSwapResponse {
        signatures: response,
    }))
}

#[utoipa::path(
        get,
        path = "/v1/keys",
        responses(
            (status = 200, description = "get keys", body = [KeysResponse])
        )
    )]
#[instrument(skip(mint), err)]
pub async fn get_keys(State(mint): State<Mint>) -> Result<Json<KeysResponse>, MokshaMintError> {
    Ok(Json(KeysResponse {
        keysets: vec![KeyResponse {
            id: mint.keyset.keyset_id.clone(),
            unit: CurrencyUnit::Sat,
            keys: mint.keyset.public_keys,
        }],
    }))
}

#[utoipa::path(
        get,
        path = "/v1/keys/{id}",
        responses(
            (status = 200, description = "get keys by id", body = [KeysResponse])
        ),
        params(
            ("id" = String, Path, description = "keyset id"),
        )
    )]
#[instrument(skip(mint), err)]
pub async fn get_keys_by_id(
    Path(id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<KeysResponse>, MokshaMintError> {
    if id != mint.keyset.keyset_id {
        return Err(MokshaMintError::KeysetNotFound(id));
    }

    Ok(Json(KeysResponse {
        keysets: vec![KeyResponse {
            id: mint.keyset.keyset_id.clone(),
            unit: CurrencyUnit::Sat,
            keys: mint.keyset.public_keys,
        }],
    }))
}

#[utoipa::path(
        get,
        path = "/v1/keysets",
        responses(
            (status = 200, description = "get keysets", body = [V1Keysets])
        ),
    )]
#[instrument(skip(mint), err)]
pub async fn get_keysets(State(mint): State<Mint>) -> Result<Json<V1Keysets>, MokshaMintError> {
    Ok(Json(V1Keysets::new(
        mint.keyset.keyset_id,
        CurrencyUnit::Sat,
        true,
    )))
}

#[utoipa::path(
        post,
        path = "/v1/mint/quote/bolt11",
        request_body = PostMintQuoteBolt11Request,
        responses(
            (status = 200, description = "post mint quote", body = [PostMintQuoteBolt11Response])
        ),
    )]
#[instrument(name = "post_mint_quote_bolt11", skip(mint), err)]
pub async fn post_mint_quote_bolt11(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteBolt11Request>,
) -> Result<Json<PostMintQuoteBolt11Response>, MokshaMintError> {
    // FIXME check currency unit
    let key = Uuid::new_v4();
    let (pr, _hash) = mint.create_invoice(key.to_string(), request.amount).await?;

    let quote = Bolt11MintQuote {
        quote_id: key,
        payment_request: pr.clone(),
        expiry: quote_expiry(), // FIXME use timestamp type in DB
        paid: false,
    };

    mint.db.add_bolt11_mint_quote(&quote).await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
        post,
        path = "/v1/mint/bolt11/{quote_id}",
        request_body = PostMintBolt11Request,
        responses(
            (status = 200, description = "post mint quote", body = [PostMintBolt11Response])
        ),
        params(
            ("quote_id" = String, Path, description = "quote id"),
        )
    )]
#[instrument(name = "post_mint_bolt11", skip(mint), err)]
pub async fn post_mint_bolt11(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBolt11Request>,
) -> Result<Json<PostMintBolt11Response>, MokshaMintError> {
    let signatures = mint
        .mint_tokens(
            PaymentMethod::Bolt11,
            request.quote.clone(),
            &request.outputs,
            &mint.keyset,
            false,
        )
        .await?;

    let old_quote = &mint
        .db
        .get_bolt11_mint_quote(&Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_bolt11_mint_quote(&Bolt11MintQuote {
            paid: true,
            ..old_quote.clone()
        })
        .await?;
    Ok(Json(PostMintBolt11Response { signatures }))
}

#[utoipa::path(
        post,
        path = "/v1/melt/quote/bolt11",
        request_body = PostMeltQuoteBolt11Request,
        responses(
            (status = 200, description = "post mint quote", body = [PostMeltQuoteBolt11Response])
        ),
    )]
#[instrument(name = "post_melt_quote_bolt11", skip(mint), err)]
pub async fn post_melt_quote_bolt11(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltQuoteBolt11Request>,
) -> Result<Json<PostMeltQuoteBolt11Response>, MokshaMintError> {
    let invoice = mint
        .lightning
        .decode_invoice(melt_request.request.clone())
        .await?;
    let amount = invoice.amount_milli_satoshis().ok_or_else(|| {
        crate::error::MokshaMintError::InvalidAmount("invalid invoice".to_owned())
    })?;
    let fee_reserve = mint.fee_reserve(amount) / 1_000; // FIXME check if this is correct
    info!("fee_reserve: {}", fee_reserve);

    let amount_sat = amount / 1_000;
    let key = Uuid::new_v4();
    let quote = Bolt11MeltQuote {
        quote_id: key,
        amount: amount_sat,
        fee_reserve,
        expiry: quote_expiry(),
        payment_request: melt_request.request.clone(),
        paid: false,
    };
    mint.db.add_bolt11_melt_quote(&quote).await?;

    Ok(Json(quote.into()))
}

fn quote_expiry() -> u64 {
    // FIXME add config option for expiry
    let now = Utc::now() + Duration::minutes(30);
    now.timestamp() as u64
}

#[utoipa::path(
        post,
        path = "/v1/melt/bolt11",
        request_body = PostMeltBolt11Request,
        responses(
            (status = 200, description = "post melt", body = [PostMeltBolt11Response])
        ),
    )]
#[instrument(name = "post_melt_bolt11", skip(mint), err)]
pub async fn post_melt_bolt11(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltBolt11Request>,
) -> Result<Json<PostMeltBolt11Response>, MokshaMintError> {
    let quote = mint
        .db
        .get_bolt11_melt_quote(&Uuid::from_str(melt_request.quote.as_str())?)
        .await?;

    info!("post_melt_bolt11 fee_reserve: {:#?}", &quote);

    let (paid, payment_preimage, change) = mint
        .melt_bolt11(
            quote.payment_request.to_owned(),
            quote.fee_reserve,
            &melt_request.inputs,
            &melt_request.outputs,
            &mint.keyset,
        )
        .await?;
    mint.db
        .update_bolt11_melt_quote(&Bolt11MeltQuote { paid, ..quote })
        .await?;

    Ok(Json(PostMeltBolt11Response {
        paid,
        payment_preimage: Some(payment_preimage),
        change,
    }))
}

#[utoipa::path(
        get,
        path = "/v1/mint/quote/bolt11/{quote_id}",
        responses(
            (status = 200, description = "get mint quote by id", body = [PostMintQuoteBolt11Response])
        ),
        params(
            ("quote_id" = String, Path, description = "quote id"),
        )
    )]
#[instrument(name = "get_mint_quote_bolt11", skip(mint), err)]
pub async fn get_mint_quote_bolt11(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMintQuoteBolt11Response>, MokshaMintError> {
    info!("get_quote: {}", quote_id);

    let quote = mint
        .db
        .get_bolt11_mint_quote(&Uuid::from_str(quote_id.as_str())?)
        .await?;

    let paid = mint
        .lightning
        .is_invoice_paid(quote.payment_request.clone())
        .await?;

    Ok(Json(Bolt11MintQuote { paid, ..quote }.into()))
}

#[utoipa::path(
        get,
        path = "/v1/melt/quote/bolt11/{quote_id}",
        responses(
            (status = 200, description = "post mint quote", body = [PostMeltQuoteBolt11Response])
        ),
        params(
            ("quote_id" = String, Path, description = "quote id"),
        )
    )]
#[instrument(name = "get_melt_quote_bolt11", skip(mint), err)]
pub async fn get_melt_quote_bolt11(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMeltQuoteBolt11Response>, MokshaMintError> {
    info!("get_melt_quote: {}", quote_id);
    let quote = mint
        .db
        .get_bolt11_melt_quote(&Uuid::from_str(quote_id.as_str())?)
        .await?;

    // FIXME check for paid?
    Ok(Json(quote.into()))
}

#[utoipa::path(
        get,
        path = "/v1/info",
        responses(
            (status = 200, description = "get mint info", body = [MintInfoResponse])
        )
    )]
#[instrument(name = "get_info", skip(mint), err)]
pub async fn get_info(State(mint): State<Mint>) -> Result<Json<MintInfoResponse>, MokshaMintError> {
    // TODO implement From-trait

    let contact = mint
        .config
        .clone()
        .info
        .contact_email
        .map(|contact| vec![vec!["email".to_owned(), contact]]);
    // FIXME

    let mint_info = MintInfoResponse {
        nuts: get_nuts(&mint.config),
        name: mint.config.info.name,
        pubkey: mint.keyset.mint_pubkey,
        version: match mint.config.info.version {
            true => Some(mint.build_params.full_version()),
            _ => None,
        },
        description: mint.config.info.description,
        description_long: mint.config.info.description_long,
        contact,
        motd: mint.config.info.motd,
    };
    Ok(Json(mint_info))
}

fn get_nuts(cfg: &MintConfig) -> Nuts {
    let default_config = BtcOnchainConfig::default();
    let config = cfg.btconchain_backend.as_ref().unwrap_or(&default_config);
    Nuts {
        nut14: Some(config.to_owned().into()),
        nut15: Some(config.to_owned().into()),
        ..Nuts::default()
    }
}
