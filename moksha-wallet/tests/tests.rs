use std::collections::HashMap;

use async_trait::async_trait;
use moksha_core::model::{
    BlindedMessage, CheckFeesResponse, Keysets, MintKeyset, PaymentRequest, PostMeltResponse,
    PostMintResponse, PostSplitResponse, Proofs, TokenV3,
};
use moksha_wallet::localstore::LocalStore;
use moksha_wallet::wallet::WalletBuilder;
use moksha_wallet::{client::Client, error::MokshaWalletError, sqlx_localstore::SqliteLocalStore};
use reqwest::Url;
use secp256k1::PublicKey;

#[derive(Clone, Default)]
struct MockClient {
    split_response: PostSplitResponse,
    post_mint_response: PostMintResponse,
    post_melt_response: PostMeltResponse,
    mint_keys: HashMap<u64, PublicKey>,
    keysets: Keysets,
}

impl MockClient {
    fn with(
        post_melt_response: PostMeltResponse,
        post_split_response: PostSplitResponse,
        mint_keys: HashMap<u64, PublicKey>,
        keysets: Keysets,
    ) -> Self {
        Self {
            mint_keys,
            keysets,
            post_melt_response,
            split_response: post_split_response,
            ..Default::default()
        }
    }
}

#[async_trait]
impl Client for MockClient {
    async fn post_split_tokens(
        &self,
        _mint_url: &Url,
        _amount: u64,
        _proofs: Proofs,
        _output: Vec<BlindedMessage>,
    ) -> Result<PostSplitResponse, MokshaWalletError> {
        Ok(self.split_response.clone())
    }

    async fn post_mint_payment_request(
        &self,
        _mint_url: &Url,
        _hash: String,
        _blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse, MokshaWalletError> {
        Ok(self.post_mint_response.clone())
    }

    async fn post_melt_tokens(
        &self,
        _mint_url: &Url,
        _proofs: Proofs,
        _pr: String,
        _outputs: Vec<BlindedMessage>,
    ) -> Result<PostMeltResponse, MokshaWalletError> {
        Ok(self.post_melt_response.clone())
    }

    async fn post_checkfees(
        &self,
        _mint_url: &Url,
        _pr: String,
    ) -> Result<CheckFeesResponse, MokshaWalletError> {
        Ok(CheckFeesResponse { fee: 0 })
    }

    async fn get_mint_keys(
        &self,
        _mint_url: &Url,
    ) -> Result<HashMap<u64, PublicKey>, MokshaWalletError> {
        Ok(self.mint_keys.clone())
    }

    async fn get_mint_keysets(&self, _mint_url: &Url) -> Result<Keysets, MokshaWalletError> {
        Ok(self.keysets.clone())
    }

    async fn get_mint_payment_request(
        &self,
        _mint_url: &Url,
        _amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError> {
        unimplemented!()
    }
}

/// tests if tokens get restored, if the lightning payment is not successful
#[tokio::test]
async fn test_pay_invoice_can_not_melt() -> anyhow::Result<()> {
    let fixture = read_fixture("token_64.cashu")?; // 60 tokens (4,8,16,32)
    let tokens: TokenV3 = fixture.try_into()?;

    let tmp = tempfile::tempdir().expect("Could not create tmp dir for wallet");
    let tmp_dir = tmp
        .path()
        .to_str()
        .expect("Could not create tmp dir for wallet");

    let localstore = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db"))
        .await
        .expect("Could not create localstore");
    localstore.migrate().await;

    localstore.add_proofs(&tokens.proofs()).await?;
    assert_eq!(64, localstore.get_proofs().await?.total_amount());

    let melt_response = read_fixture("post_melt_response_not_paid.json")?;
    let split_response = read_fixture("post_split_response_24_40.json")?;
    let mint_keyset = MintKeyset::new("mysecret".to_string(), "".to_string());
    let keysets = Keysets::new(vec![mint_keyset.keyset_id]);

    let mock_client = MockClient::with(
        serde_json::from_str::<PostMeltResponse>(&melt_response)?,
        serde_json::from_str::<PostSplitResponse>(&split_response)?,
        mint_keyset.public_keys,
        keysets,
    );

    let wallet = WalletBuilder::default()
        .with_client(mock_client)
        .with_localstore(localstore.clone())
        .with_mint_url(Url::parse("http://localhost:8080").expect("invalid url"))
        .build()
        .await?;

    // 21 sats
    let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

    let result = wallet.pay_invoice(invoice).await?;
    assert!(!result.paid);
    assert_eq!(64, localstore.get_proofs().await?.total_amount());
    assert!(!result.paid);
    Ok(())
}

fn read_fixture(name: &str) -> anyhow::Result<String> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
    Ok(raw_token.trim().to_string())
}
