// use std::collections::HashMap;

// use async_trait::async_trait;
// use moksha_core::blind::BlindedMessage;
// use moksha_core::fixture::{read_fixture, read_fixture_as};
// use moksha_core::keyset::{Keysets, MintKeyset, V1Keysets};
// use moksha_core::primitives::{
//     CheckFeesResponse, CurrencyUnit, KeyResponse, KeysResponse, MintLegacyInfoResponse,
//     PaymentRequest, PostMeltBolt11Response, PostMeltResponse, PostMintResponse, PostSplitResponse,
//     PostSwapResponse,
// };
// use moksha_core::proof::Proofs;
// use moksha_core::token::TokenV3;

// use moksha_wallet::localstore::sqlite::SqliteLocalStore;
// use moksha_wallet::localstore::LocalStore;
// use moksha_wallet::wallet::WalletBuilder;
// use moksha_wallet::{client::LegacyClient, error::MokshaWalletError};
// use reqwest::Url;
// use secp256k1::PublicKey;

//FIXME

// // /// tests if tokens get restored, if the lightning payment is not successful
// #[tokio::test]
// async fn test_pay_invoice_can_not_melt() -> anyhow::Result<()> {
//     let fixture = read_fixture("token_64.cashu")?; // 60 tokens (4,8,16,32)
//     let tokens: TokenV3 = fixture.try_into()?;

//     let tmp = tempfile::tempdir().expect("Could not create tmp dir for wallet");
//     let tmp_dir = tmp
//         .path()
//         .to_str()
//         .expect("Could not create tmp dir for wallet");

//     let localstore = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db"))
//         .await
//         .expect("Could not create localstore");

//     localstore.add_proofs(&tokens.proofs()).await?;
//     assert_eq!(64, localstore.get_proofs().await?.total_amount());

//     let melt_response =
//         read_fixture_as::<PostMeltBolt11Response>("post_melt_response_not_paid.json")?;
//     let split_response = read_fixture_as::<PostSwapResponse>("post_split_response_24_40.json")?;

//     let mut mock_client = create_mock();

//     let wallet = WalletBuilder::default()
//         .with_client(mock_client)
//         .with_localstore(localstore.clone())
//         .with_mint_url(Url::parse("http://localhost:8080").expect("invalid url"))
//         .build()
//         .await?;

//     // 21 sats
//     let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

//     let result = wallet.pay_invoice(invoice).await?;
//     assert!(!result.paid);
//     assert_eq!(64, localstore.get_proofs().await?.total_amount());
//     assert!(!result.paid);
//     Ok(())
// }
