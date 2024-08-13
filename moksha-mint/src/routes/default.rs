use axum::{
    extract::{Path, State},
    Json,
};
use moksha_core::{
    keyset::Keysets,
    primitives::{
        Bolt11MeltQuote, Bolt11MintQuote, CurrencyUnit, KeyResponse, KeysResponse,
        MintInfoResponse, Nuts, PaymentMethod, PostMeltBolt11Request, PostMeltBolt11Response,
        PostMeltQuoteBolt11Request, PostMeltQuoteBolt11Response, PostMintBolt11Request,
        PostMintBolt11Response, PostMintQuoteBolt11Request, PostMintQuoteBolt11Response,
        PostSwapRequest, PostSwapResponse,
    },
};
use std::fs::File;
use std::io::Write;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::database::Database;
use crate::{
    config::{BtcOnchainConfig, MintConfig},
    error::MokshaMintError,
    mint::Mint,
};
use chrono::{Duration, Utc};
use moksha_core::primitives::{
    BillKeys, BitcreditMintQuote, BitcreditQuoteCheck, BitcreditRequestToMint,
    CheckBitcreditQuoteResponse, ParamsBitcreditQuoteCheck, PostMintBitcreditRequest,
    PostMintBitcreditResponse, PostMintQuoteBitcreditRequest, PostMintQuoteBitcreditResponse,
    PostRequestToMintBitcreditRequest, PostRequestToMintBitcreditResponse,
};
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
            (status = 200, description = "get keysets", body = [Keysets])
        ),
    )]
#[instrument(skip(mint), err)]
pub async fn get_keysets(State(mint): State<Mint>) -> Result<Json<Keysets>, MokshaMintError> {
    Ok(Json(Keysets::new(
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

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_bolt11_mint_quote(&mut tx, &quote).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
    post,
    path = "/v1/mint/quote/bitcredit",
    request_body = PostMintQuoteBitcreditRequest,
    responses(
    (status = 200, description = "post mint quote", body = [PostMintQuoteBitcreditResponse])
    ),
)]
#[instrument(name = "post_mint_quote_bitcredit", skip(mint), err)]
pub async fn post_mint_quote_bitcredit(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteBitcreditRequest>,
) -> Result<Json<PostMintQuoteBitcreditResponse>, MokshaMintError> {
    // FIXME check currency unit
    let key = Uuid::new_v4();

    let quote = BitcreditMintQuote {
        quote_id: key,
        bill_id: request.bill_id,
        node_id: request.node_id,
        sent: false,
        amount: request.amount,
        endorsed: false,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_bitcredit_mint_quote(&mut tx, &quote).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
    post,
    path = "/v1/mint/request/bitcredit",
    request_body = PostRequestToMintBitcreditRequest,
    responses(
    (status = 200, description = "post request to mint", body = [PostRequestToMintBitcreditResponse])
    ),
)]
#[instrument(name = "post_request_to_mint_bitcredit", skip(mint), err)]
pub async fn post_request_to_mint_bitcredit(
    State(mint): State<Mint>,
    Json(request): Json<PostRequestToMintBitcreditRequest>,
    //TODO: correct response
) -> Result<Json<PostRequestToMintBitcreditResponse>, MokshaMintError> {
    let request_to_mint = BitcreditRequestToMint {
        bill_key: request.bill_keys.private_key_pem.clone(),
        bill_id: request.bill_id.clone(),
    };

    write_bill_keys_to_file(
        request.bill_id,
        request.bill_keys.private_key_pem,
        request.bill_keys.public_key_pem,
    );

    let mut tx = mint.db.begin_tx().await?;
    mint.db
        .add_bitcredit_request_to_mint(&mut tx, &request_to_mint)
        .await?;
    tx.commit().await?;
    Ok(Json(request_to_mint.into()))
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
#[instrument(name = "post_mint_bolt11", fields(quote_id = %request.quote), skip_all, err)]
pub async fn post_mint_bolt11(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBolt11Request>,
) -> Result<Json<PostMintBolt11Response>, MokshaMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let signatures = mint
        .mint_tokens(
            &mut tx,
            PaymentMethod::Bolt11,
            request.quote.clone(),
            &request.outputs,
            &mint.keyset,
            false,
        )
        .await?;

    let old_quote = &mint
        .db
        .get_bolt11_mint_quote(&mut tx, &Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_bolt11_mint_quote(
            &mut tx,
            &Bolt11MintQuote {
                paid: true,
                ..old_quote.clone()
            },
        )
        .await?;
    tx.commit().await?;
    Ok(Json(PostMintBolt11Response { signatures }))
}

#[utoipa::path(
    post,
    path = "/v1/mint/bitcredit/{quote_id}",
    request_body = PostMintBitcreditRequest,
    responses(
    (status = 200, description = "post mint quote bitcredit", body = [PostMintBitcreditResponse])
    ),
    params(
    ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "post_mint_bitcredit", fields(quote_id = %request.quote), skip_all, err)]
pub async fn post_mint_bitcredit(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBitcreditRequest>,
) -> Result<Json<PostMintBitcreditResponse>, MokshaMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let signatures = mint
        .mint_tokens(
            &mut tx,
            PaymentMethod::Bitcredit,
            request.quote.clone(),
            &request.outputs,
            &mint.keyset,
            false,
        )
        .await?;

    let old_quote = &mint
        .db
        .get_bitcredit_mint_quote(&mut tx, &Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_bitcredit_mint_quote(
            &mut tx,
            &BitcreditMintQuote {
                endorsed: true,
                sent: true,
                ..old_quote.clone()
            },
        )
        .await?;
    tx.commit().await?;
    Ok(Json(PostMintBitcreditResponse { signatures }))
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
    let fee_reserve = mint.fee_reserve_msat(amount) / 1_000; // FIXME check if this is correct
    debug!("fee_reserve: {}", fee_reserve);

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
    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_bolt11_melt_quote(&mut tx, &quote).await?;
    tx.commit().await?;

    Ok(Json(quote.into()))
}

fn quote_expiry() -> u64 {
    // FIXME add config option for expiry
    let now = Utc::now() + Duration::try_minutes(30).expect("invalid duration");
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
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_bolt11_melt_quote(&mut tx, &Uuid::from_str(melt_request.quote.as_str())?)
        .await?;

    debug!("post_melt_bolt11 fee_reserve: {:#?}", &quote);

    let (paid, payment_preimage, change) = mint
        .melt_bolt11(
            &mut tx,
            quote.payment_request.to_owned(),
            quote.fee_reserve,
            &melt_request.inputs,
            melt_request.outputs,
            &mint.keyset,
        )
        .await?;
    mint.db
        .update_bolt11_melt_quote(&mut tx, &Bolt11MeltQuote { paid, ..quote })
        .await?;
    tx.commit().await?;

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
    debug!("get_quote: {}", quote_id);

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_bolt11_mint_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    tx.commit().await?;

    let paid = mint
        .lightning
        .is_invoice_paid(quote.payment_request.clone())
        .await?;

    Ok(Json(Bolt11MintQuote { paid, ..quote }.into()))
}

#[utoipa::path(
    get,
    path = "/v1/quote/bitcredit/check/{bill_id}/{node_id}",
    responses(
    (status = 200, description = "check bitcredit quote", body = [CheckBitcreditQuoteResponse])
    ),
)]
#[instrument(name = "check_bitcredit_quote", skip(mint), err)]
pub async fn check_bitcredit_quote(
    params: Path<ParamsBitcreditQuoteCheck>,
    State(mint): State<Mint>,
) -> Result<Json<CheckBitcreditQuoteResponse>, MokshaMintError> {
    let quote_check = BitcreditQuoteCheck {
        node_id: params.node_id.clone(),
        bill_id: params.bill_id.clone(),
    };

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint.db.check_bitcredit_quote(&mut tx, &quote_check).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
    get,
    path = "/v1/mint/quote/bitcredit/{quote_id}",
    responses(
        (status = 200, description = "get bitcredit mint quote by id", body = [PostMintQuoteBitcreditResponse])
    ),
    params(
        ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "get_mint_quote_bitcredit", skip(mint), err)]
pub async fn get_mint_quote_bitcredit(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMintQuoteBitcreditResponse>, MokshaMintError> {
    debug!("get_quote: {}", quote_id);

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_bitcredit_mint_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    tx.commit().await?;

    Ok(Json(BitcreditMintQuote { ..quote }.into()))
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
    debug!("get_melt_quote: {}", quote_id);
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_bolt11_melt_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;

    tx.commit().await?;
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

    let mint_info = mint.config.info.clone();
    let contact = Some(
        vec![
            mint_info
                .contact_email
                .map(|email| vec!["email".to_owned(), email]),
            mint_info
                .contact_twitter
                .map(|twitter| vec!["twitter".to_owned(), twitter]),
            mint_info
                .contact_nostr
                .map(|nostr| vec!["nostr".to_owned(), nostr]),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<Vec<String>>>(),
    );

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
        nut17: Some(config.to_owned().into()),
        nut18: Some(config.to_owned().into()),
        ..Nuts::default()
    }
}

fn write_bill_keys_to_file(bill_name: String, private_key: String, public_key: String) {
    let keys: BillKeys = BillKeys {
        private_key_pem: private_key,
        public_key_pem: public_key,
    };

    //TODO: this static path only for testing. Remove it
    let output_path = "/home/mtbitcr/RustroverProjects/E-Bills/bills_keys".to_string()
        + "/"
        + bill_name.as_str()
        + ".json";
    let mut file = File::create(output_path.clone()).unwrap();
    file.write(serde_json::to_string_pretty(&keys).unwrap().as_bytes())
        .unwrap();
}
