use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use moksha_core::{
    keyset::{generate_hash, Keysets},
    primitives::{
        CheckFeesRequest, CheckFeesResponse, PaymentMethod, PaymentRequest, PostMeltRequest,
        PostMeltResponse, PostMintRequest, PostMintResponse, PostSplitRequest, PostSplitResponse,
    },
};
use secp256k1::PublicKey;
use tracing::{event, Level};

use crate::{
    error::MokshaMintError,
    mint::Mint,
    model::{GetMintQuery, PostMintQuery},
};

pub async fn get_legacy_keys(
    State(mint): State<Mint>,
) -> Result<Json<HashMap<u64, PublicKey>>, MokshaMintError> {
    Ok(Json(mint.keyset_legacy.public_keys))
}

pub async fn get_legacy_keysets(
    State(mint): State<Mint>,
) -> Result<Json<Keysets>, MokshaMintError> {
    Ok(Json(Keysets::new(vec![mint.keyset_legacy.keyset_id])))
}

pub async fn get_legacy_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<GetMintQuery>,
) -> Result<Json<PaymentRequest>, MokshaMintError> {
    let (pr, hash) = mint
        .create_invoice(generate_hash(), mint_query.amount)
        .await?;
    Ok(Json(PaymentRequest { pr, hash }))
}

pub async fn post_legacy_mint(
    State(mint): State<Mint>,
    Query(mint_query): Query<PostMintQuery>,
    Json(blinded_messages): Json<PostMintRequest>,
) -> Result<Json<PostMintResponse>, MokshaMintError> {
    event!(
        Level::INFO,
        "post_mint: {mint_query:#?} {blinded_messages:#?}"
    );

    let promises = mint
        .mint_tokens(
            PaymentMethod::Bolt11,
            mint_query.hash,
            &blinded_messages.outputs,
            &mint.keyset_legacy,
            true,
        )
        .await?;
    Ok(Json(PostMintResponse { promises }))
}

pub async fn post_legacy_split(
    State(mint): State<Mint>,
    Json(swap_request): Json<PostSplitRequest>,
) -> Result<Json<PostSplitResponse>, MokshaMintError> {
    let response = mint
        .swap(
            &swap_request.proofs,
            &swap_request.outputs,
            &mint.keyset_legacy,
        )
        .await?;

    Ok(Json(PostSplitResponse::with_promises(response)))
}

pub async fn post_legacy_melt(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltRequest>,
) -> Result<Json<PostMeltResponse>, MokshaMintError> {
    let (paid, preimage, change) = mint
        .melt_bolt11(
            melt_request.pr,
            0, // FIXME set correct fee reserve for legacy api
            &melt_request.proofs,
            &melt_request.outputs,
            &mint.keyset_legacy,
        )
        .await?;

    Ok(Json(PostMeltResponse {
        paid,
        preimage,
        change,
    }))
}

pub async fn post_legacy_check_fees(
    State(mint): State<Mint>,
    Json(_check_fees): Json<CheckFeesRequest>,
) -> Result<Json<CheckFeesResponse>, MokshaMintError> {
    let invoice = mint.lightning.decode_invoice(_check_fees.pr).await?;

    Ok(Json(CheckFeesResponse {
        fee: mint.fee_reserve(invoice.amount_milli_satoshis().ok_or_else(|| {
            crate::error::MokshaMintError::InvalidAmount("invalid invoice".to_owned())
        })?),
    }))
}
