use axum::{
    extract::{Path, State},
    Json,
};
use moksha_core::primitives::{
    BtcOnchainMeltQuote, BtcOnchainMintQuote, CurrencyUnit, PaymentMethod,
    PostMeltBtcOnchainRequest, PostMeltBtcOnchainResponse, PostMeltQuoteBtcOnchainRequest,
    PostMeltQuoteBtcOnchainResponse, PostMintBtcOnchainRequest, PostMintBtcOnchainResponse,
    PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse,
};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::database::Database;
use crate::{error::MokshaMintError, mint::Mint};
use chrono::{Duration, Utc};
use std::str::FromStr;

#[utoipa::path(
        post,
        path = "/v1/mint/quote/btconchain",
        request_body = PostMintQuoteOnchainRequest,
        responses(
            (status = 200, description = "post mint quote", body = [PostMintQuoteOnchainResponse])
        ),
    )]
#[instrument(name = "post_mint_quote_btconchain", skip(mint), err)]
pub async fn post_mint_quote_btconchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteBtcOnchainRequest>,
) -> Result<Json<PostMintQuoteBtcOnchainResponse>, MokshaMintError> {
    let onchain_config = mint.config.btconchain_backend.unwrap_or_default();

    if request.unit != CurrencyUnit::Sat {
        return Err(MokshaMintError::CurrencyNotSupported(request.unit));
    }

    if request.amount < onchain_config.min_amount {
        return Err(MokshaMintError::InvalidAmount(format!(
            "amount is too low. Min amount is {}",
            onchain_config.min_amount
        )));
    }

    if request.amount > onchain_config.max_amount {
        return Err(MokshaMintError::InvalidAmount(format!(
            "amount is too high. Max amount is {}",
            onchain_config.max_amount
        )));
    }

    let quote_id = Uuid::new_v4();
    let address = mint
        .onchain
        .as_ref()
        .expect("onchain backend not configured")
        .new_address()
        .await?;

    let quote = BtcOnchainMintQuote {
        quote_id,
        address,
        unit: request.unit,
        amount: request.amount,
        expiry: quote_onchain_expiry(),
        paid: false,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_mint_quote(&mut tx, &quote).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
        get,
        path = "/v1/mint/quote/btconchain/{quote_id}",
        responses(
            (status = 200, description = "get mint quote by id", body = [PostMintQuoteOnchainResponse])
        ),
        params(
            ("quote_id" = String, Path, description = "quote id"),
        )
    )]
#[instrument(name = "get_mint_quote_btconchain", skip(mint), err)]
pub async fn get_mint_quote_btconchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMintQuoteBtcOnchainResponse>, MokshaMintError> {
    info!("get_quote onchain: {}", quote_id);

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    tx.commit().await?;

    let min_confs = mint
        .config
        .btconchain_backend
        .unwrap_or_default()
        .min_confirmations;

    let paid = mint
        .onchain
        .as_ref()
        .expect("onchain backend not configured")
        .is_paid(&quote.address, quote.amount, min_confs)
        .await?;

    Ok(Json(BtcOnchainMintQuote { paid, ..quote }.into()))
}

#[utoipa::path(
        post,
        path = "/v1/mint/btconchain",
        request_body = PostMintOnchainRequest,
        responses(
            (status = 200, description = "post mint", body = [PostMintOnchainResponse])
        ),
    )]
#[instrument(name = "post_mint_btconchain", skip(mint), err)]
pub async fn post_mint_btconchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBtcOnchainRequest>,
) -> Result<Json<PostMintBtcOnchainResponse>, MokshaMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let signatures = mint
        .mint_tokens(
            &mut tx,
            PaymentMethod::BtcOnchain,
            request.quote.clone(),
            &request.outputs,
            &mint.keyset,
            false,
        )
        .await?;

    let old_quote = &mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_onchain_mint_quote(
            &mut tx,
            &BtcOnchainMintQuote {
                paid: true,
                ..old_quote.clone()
            },
        )
        .await?;
    tx.commit().await?;
    Ok(Json(PostMintBtcOnchainResponse { signatures }))
}

#[utoipa::path(
        post,
        path = "/v1/melt/quote/btconchain",
        request_body = PostMeltQuoteOnchainRequest,
        responses(
            (status = 200, description = "post mint quote", body = [Vec<PostMeltQuoteOnchainResponse>])
        ),
    )]
#[instrument(name = "post_melt_quote_btconchain", skip(mint), err)]
pub async fn post_melt_quote_btconchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltQuoteBtcOnchainRequest>,
) -> Result<Json<Vec<PostMeltQuoteBtcOnchainResponse>>, MokshaMintError> {
    let PostMeltQuoteBtcOnchainRequest {
        address,
        amount,
        unit,
    } = melt_request;

    let onchain_config = mint.config.btconchain_backend.unwrap_or_default();

    if unit != CurrencyUnit::Sat {
        return Err(MokshaMintError::CurrencyNotSupported(unit));
    }

    if amount < onchain_config.min_amount {
        return Err(MokshaMintError::InvalidAmount(format!(
            "amount is too low. Min amount is {}",
            onchain_config.min_amount
        )));
    }

    if amount > onchain_config.max_amount {
        return Err(MokshaMintError::InvalidAmount(format!(
            "amount is too high. Max amount is {}",
            onchain_config.max_amount
        )));
    }

    let fee_response = mint
        .onchain
        .as_ref()
        .expect("onchain backend not configured")
        .estimate_fee(&address, amount)
        .await?;

    info!("post_melt_quote_onchain fee_reserve: {:#?}", &fee_response);

    let quote = BtcOnchainMeltQuote {
        quote_id: Uuid::new_v4(),
        address,
        amount,
        fee_total: fee_response.fee_in_sat,
        fee_sat_per_vbyte: fee_response.sat_per_vbyte,
        expiry: quote_onchain_expiry(),
        paid: false,
        description: Some(format!("{} sat per vbyte", fee_response.sat_per_vbyte)),
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_melt_quote(&mut tx, &quote).await?;
    tx.commit().await?;

    Ok(Json(vec![quote.into()]))
}

#[utoipa::path(
        get,
        path = "/v1/melt/quote/btconchain/{quote_id}",
        responses(
            (status = 200, description = "post mint quote", body = [PostMeltQuoteOnchainResponse])
        ),
        params(
            ("quote_id" = String, Path, description = "quote id"),
        )
    )]
#[instrument(name = "get_melt_quote_btconchain", skip(mint), err)]
pub async fn get_melt_quote_btconchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMeltQuoteBtcOnchainResponse>, MokshaMintError> {
    info!("get_melt_quote onchain: {}", quote_id);
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;

    let paid = is_onchain_paid(&mint, &quote).await?;
    if paid {
        mint.db
            .update_onchain_melt_quote(
                &mut tx,
                &BtcOnchainMeltQuote {
                    paid,
                    ..quote.clone()
                },
            )
            .await?;
    }

    Ok(Json(BtcOnchainMeltQuote { paid, ..quote }.into()))
}

#[utoipa::path(
        post,
        path = "/v1/melt/btconchain",
        request_body = PostMeltOnchainRequest,
        responses(
            (status = 200, description = "post melt", body = [PostMeltOnchainResponse])
        ),
    )]
#[instrument(name = "post_melt_btconchain", skip(mint), err)]
pub async fn post_melt_btconchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltBtcOnchainRequest>,
) -> Result<Json<PostMeltBtcOnchainResponse>, MokshaMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(melt_request.quote.as_str())?)
        .await?;

    let txid = mint.melt_onchain(&quote, &melt_request.inputs).await?;
    let paid = is_onchain_paid(&mint, &quote).await?;

    mint.db
        .update_onchain_melt_quote(&mut tx, &BtcOnchainMeltQuote { paid, ..quote })
        .await?;
    tx.commit().await?;

    Ok(Json(PostMeltBtcOnchainResponse {
        paid,
        txid: Some(txid),
    }))
}

async fn is_onchain_paid(
    mint: &Mint,
    quote: &BtcOnchainMeltQuote,
) -> Result<bool, MokshaMintError> {
    let min_confs = mint
        .config
        .btconchain_backend
        .clone()
        .unwrap_or_default()
        .min_confirmations;

    mint.onchain
        .as_ref()
        .expect("onchain backend not configured")
        .is_paid(&quote.address, quote.amount, min_confs)
        .await
}

fn quote_onchain_expiry() -> u64 {
    // FIXME add config option for expiry
    let now = Utc::now() + Duration::try_minutes(5).expect("invalid duration");
    now.timestamp() as u64
}
