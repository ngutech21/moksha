use std::{collections::HashMap, fs::create_dir, path::PathBuf};

use dirs::home_dir;
use moksha_core::{
    dhke::Dhke,
    model::{
        split_amount, Amount, BlindedMessage, BlindedSignature, Keysets, PaymentRequest,
        PostMeltResponse, Proof, Proofs, TokenV3, TotalAmount,
    },
};
use reqwest::Url;
use secp256k1::{PublicKey, SecretKey};

use crate::{
    client::Client,
    error::MokshaWalletError,
    localstore::{LocalStore, WalletKeyset},
};
use lightning_invoice::Invoice as LNInvoice;
use std::str::FromStr;

pub const ENV_DB_PATH: &str = "WALLET_DB_PATH";

pub struct Wallet {
    client: Box<dyn Client>,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
    dhke: Dhke,
    localstore: Box<dyn LocalStore>,
    mint_url: Url,
}

impl Clone for Wallet {
    fn clone(&self) -> Self {
        Self {
            mint_keys: self.mint_keys.clone(),
            keysets: self.keysets.clone(),
            dhke: self.dhke.clone(),
            mint_url: self.mint_url.clone(),
            client: dyn_clone::clone_box(&*self.client),
            localstore: dyn_clone::clone_box(&*self.localstore),
        }
    }
}

#[derive(Default)]
pub struct WalletBuilder {
    client: Option<Box<dyn Client>>,
    localstore: Option<Box<dyn LocalStore>>,
    mint_url: Option<Url>,
}

impl WalletBuilder {
    pub fn with_client(mut self, client: Box<dyn Client>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_localstore(mut self, localstore: Box<dyn LocalStore>) -> Self {
        self.localstore = Some(localstore);
        self
    }

    pub fn with_mint_url(mut self, mint_url: Url) -> Self {
        self.mint_url = Some(mint_url);
        self
    }

    pub async fn build(self) -> Result<Wallet, MokshaWalletError> {
        let client = self.client.expect("client is required");
        let localstore = self.localstore.expect("localstore is required");
        let mint_url = self.mint_url.expect("mint_url is required");

        let load_keysets = localstore.get_keysets().await?;

        let mint_keysets = client.get_mint_keysets(&mint_url).await?;
        if load_keysets.is_empty() {
            let wallet_keysets = mint_keysets
                .keysets
                .iter()
                .map(|m| WalletKeyset {
                    id: m.to_owned(),
                    mint_url: mint_url.to_string(),
                })
                .collect::<Vec<WalletKeyset>>();

            for wkeyset in wallet_keysets {
                localstore.add_keyset(&wkeyset).await?;
            }
        }

        let keys = client.get_mint_keys(&mint_url).await?;

        Ok(Wallet::new(
            client,
            keys,
            mint_keysets,
            localstore,
            mint_url,
        ))
    }
}

impl Wallet {
    fn new(
        client: Box<dyn Client>,
        mint_keys: HashMap<u64, PublicKey>,
        keysets: Keysets,
        localstore: Box<dyn LocalStore>,
        mint_url: Url,
    ) -> Self {
        Self {
            client,
            mint_keys,
            keysets,
            dhke: Dhke::new(),
            localstore,
            mint_url,
        }
    }

    pub fn builder() -> WalletBuilder {
        WalletBuilder::default()
    }

    /// Returns the path to the wallet database file.
    ///
    /// The path is determined by the value of the `WALLET_DB_PATH` environment variable. If the
    /// variable is not set, the function creates a `.moksha` directory in the user's home directory
    /// and returns a path to a `wallet.db` file in that directory.
    ///
    /// # Examples
    ///
    /// ```
    /// let db_path = moksha_wallet::wallet::Wallet::db_path();
    /// println!("Database path: {}", db_path);
    /// ```
    pub fn db_path() -> String {
        match std::env::var(ENV_DB_PATH) {
            Ok(val) => val,
            Err(_) => {
                let home = home_dir()
                    .expect("home dir not found")
                    .to_str()
                    .expect("home dir is invalid")
                    .to_owned();
                // in a sandboxed environment on mac the path looks like
                // /Users/$USER_NAME/Library/Containers/..... so we have are just ising the first 2 parts
                let home = home
                    .split('/')
                    .take(3)
                    .collect::<Vec<&str>>()
                    .join(std::path::MAIN_SEPARATOR_STR);
                let moksha_dir = format!("{}{}.moksha", home, std::path::MAIN_SEPARATOR);

                if !std::path::Path::new(&moksha_dir).exists() {
                    create_dir(std::path::Path::new(&moksha_dir))
                        .expect("failed to create .moksha dir");
                }

                format!("{moksha_dir}/wallet.db")
            }
        }
    }

    pub fn config_dir() -> PathBuf {
        let home = home_dir()
            .expect("home dir not found")
            .to_str()
            .expect("home dir is invalid")
            .to_owned();
        // in a sandboxed environment on mac the path looks like
        // /Users/$USER_NAME/Library/Containers/..... so we have are just ising the first 2 parts
        let home = home
            .split('/')
            .take(3)
            .collect::<Vec<&str>>()
            .join(std::path::MAIN_SEPARATOR_STR);
        let moksha_dir = format!("{}{}.moksha", home, std::path::MAIN_SEPARATOR);

        if !std::path::Path::new(&moksha_dir).exists() {
            create_dir(std::path::Path::new(&moksha_dir)).expect("failed to create .moksha dir");
        }
        PathBuf::from(moksha_dir)
    }

    pub async fn get_mint_payment_request(
        &self,
        amount: u64,
    ) -> Result<PaymentRequest, MokshaWalletError> {
        self.client
            .get_mint_payment_request(&self.mint_url, amount)
            .await
    }

    pub async fn get_balance(&self) -> Result<u64, MokshaWalletError> {
        Ok(self.localstore.get_proofs().await?.total_amount())
    }

    pub async fn send_tokens(&self, amount: u64) -> Result<TokenV3, MokshaWalletError> {
        let balance = self.get_balance().await?;
        if amount > balance {
            return Err(MokshaWalletError::NotEnoughTokens);
        }

        let all_proofs = self.localstore.get_proofs().await?;
        let selected_proofs = all_proofs.proofs_for_amount(amount)?;
        let selected_tokens = (self.mint_url.as_ref().to_owned(), selected_proofs.clone()).into();

        let (remaining_tokens, result) = self.split_tokens(&selected_tokens, amount.into()).await?;

        // FIXME create transaction
        self.localstore.delete_proofs(&selected_proofs).await?;
        self.localstore
            .add_proofs(&remaining_tokens.proofs())
            .await?;

        Ok(result)
    }

    pub async fn receive_tokens(&self, tokens: &TokenV3) -> Result<(), MokshaWalletError> {
        let total_amount = tokens.total_amount();
        let (_, redeemed_tokens) = self.split_tokens(tokens, total_amount.into()).await?;
        self.localstore
            .add_proofs(&redeemed_tokens.proofs())
            .await?;
        Ok(())
    }

    pub async fn pay_invoice(
        &self,
        invoice: String,
    ) -> Result<PostMeltResponse, MokshaWalletError> {
        let all_proofs = self.localstore.get_proofs().await?;

        let fees = self
            .client
            .post_checkfees(&self.mint_url, invoice.clone())
            .await?;

        let ln_amount = Self::get_invoice_amount(&invoice)? + fees.fee;

        if ln_amount > all_proofs.total_amount() {
            return Err(MokshaWalletError::NotEnoughTokens);
        }
        let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

        let total_proofs = {
            let selected_tokens =
                (self.mint_url.as_str().to_owned(), selected_proofs.clone()).into();
            let split_result = self
                .split_tokens(&selected_tokens, ln_amount.into())
                .await?;

            // FIXME create transaction
            self.localstore.delete_proofs(&selected_proofs).await?;
            self.localstore.add_proofs(&split_result.0.proofs()).await?;

            split_result.1.proofs()
        };

        match self.melt_token(invoice, ln_amount, &total_proofs).await {
            Ok(response) => {
                if !response.paid {
                    self.localstore.add_proofs(&total_proofs).await?;
                }
                Ok(response)
            }
            Err(e) => {
                self.localstore.add_proofs(&total_proofs).await?;
                Err(e)
            }
        }
    }

    pub async fn split_tokens(
        &self,
        tokens: &TokenV3,
        splt_amount: Amount,
    ) -> Result<(TokenV3, TokenV3), MokshaWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount: Amount = (total_token_amount - splt_amount.0).into();
        let first_secrets = first_amount.split().create_secrets();
        let first_outputs = self.create_blinded_messages(first_amount.0, &first_secrets)?;

        // ############################################################################

        let second_amount = splt_amount.clone();
        let second_secrets = second_amount.split().create_secrets();
        let second_outputs = self.create_blinded_messages(second_amount.0, &second_secrets)?;

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        if tokens.total_amount() != total_outputs.total_amount() {
            return Err(MokshaWalletError::InvalidProofs);
        }

        let split_result = self
            .client
            .post_split_tokens(
                &self.mint_url,
                splt_amount.0,
                tokens.proofs(),
                total_outputs,
            )
            .await?;

        let first_tokens = (
            self.mint_url.as_ref().to_owned(),
            self.create_proofs_from_blinded_signatures(
                split_result.fst,
                first_secrets,
                first_outputs,
            )?,
        )
            .into();

        let second_tokens = (
            self.mint_url.as_ref().to_owned(),
            self.create_proofs_from_blinded_signatures(
                split_result.snd,
                second_secrets,
                second_outputs,
            )?,
        )
            .into();

        Ok((first_tokens, second_tokens))
    }

    async fn melt_token(
        &self,
        pr: String,
        _invoice_amount: u64,
        proofs: &Proofs,
    ) -> Result<PostMeltResponse, MokshaWalletError> {
        let melt_response = self
            .client
            .post_melt_tokens(&self.mint_url, proofs.clone(), pr, vec![])
            .await?;

        if melt_response.paid {
            self.localstore.delete_proofs(proofs).await?;
        }
        Ok(melt_response)
    }

    fn decode_invoice(payment_request: &str) -> Result<LNInvoice, MokshaWalletError> {
        LNInvoice::from_str(payment_request)
            .map_err(|err| MokshaWalletError::DecodeInvoice(payment_request.to_owned(), err))
    }

    fn get_invoice_amount(payment_request: &str) -> Result<u64, MokshaWalletError> {
        let invoice = Self::decode_invoice(payment_request)?;
        Ok(invoice
            .amount_milli_satoshis()
            .ok_or_else(|| MokshaWalletError::InvalidInvoice(payment_request.to_owned()))?
            / 1000)
    }

    pub async fn mint_tokens(
        &self,
        amount: Amount,
        hash: String,
    ) -> Result<TokenV3, MokshaWalletError> {
        let split_amount = amount.split();
        let secrets = split_amount.create_secrets();

        let blinded_messages = split_amount
            .0
            .into_iter()
            .zip(secrets.clone())
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>();

        let post_mint_resp = self
            .client
            .post_mint_payment_request(
                &self.mint_url,
                hash,
                blinded_messages
                    .clone()
                    .into_iter()
                    .map(|(msg, _)| msg)
                    .collect::<Vec<BlindedMessage>>(),
            )
            .await?;

        // step 3: unblind signatures
        let current_keyset = self.keysets.get_current_keyset(&self.mint_keys)?;

        let private_keys = blinded_messages
            .clone()
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        let proofs = post_mint_resp
            .promises
            .iter()
            .zip(private_keys)
            .zip(secrets)
            .map(|((p, priv_key), secret)| {
                let key = self
                    .mint_keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset.clone())
            })
            .collect::<Vec<Proof>>()
            .into();

        let tokens: TokenV3 = (self.mint_url.as_ref().to_owned(), proofs).into();
        self.localstore.add_proofs(&tokens.proofs()).await?;

        Ok(tokens)
    }

    // FIXME implement for Amount
    fn create_blinded_messages(
        &self,
        amount: u64,
        secrets: &[String],
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, MokshaWalletError> {
        let split_amount = split_amount(amount);

        Ok(split_amount
            .into_iter()
            .zip(secrets)
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) =
                    self.dhke.step1_alice(secret.to_string(), None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>())
    }

    fn create_proofs_from_blinded_signatures(
        &self,
        signatures: Vec<BlindedSignature>,
        secrets: Vec<String>,
        outputs: Vec<(BlindedMessage, SecretKey)>,
    ) -> Result<Proofs, MokshaWalletError> {
        let current_keyset = self.keysets.get_current_keyset(&self.mint_keys)?;

        let private_keys = outputs
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        Ok(signatures
            .iter()
            .zip(private_keys)
            .zip(secrets)
            .map(|((p, priv_key), secret)| {
                let key = self
                    .mint_keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset.clone())
            })
            .collect::<Vec<Proof>>()
            .into())
    }
}

// FIXME implement for Vec<BlindedMessage, Secretkey>
fn get_blinded_msg(blinded_messages: Vec<(BlindedMessage, SecretKey)>) -> Vec<BlindedMessage> {
    blinded_messages
        .into_iter()
        .map(|(msg, _)| msg)
        .collect::<Vec<BlindedMessage>>()
}

#[cfg(test)]
mod tests {
    use super::Wallet;
    use crate::{
        client::Client,
        error::MokshaWalletError,
        localstore::{LocalStore, WalletKeyset},
    };
    use async_trait::async_trait;
    use moksha_core::model::{
        BlindedMessage, CheckFeesResponse, Keysets, MintKeyset, PaymentRequest, PostMeltResponse,
        PostMintResponse, PostSplitResponse, Proofs, Token, TokenV3,
    };
    use reqwest::Url;
    use secp256k1::PublicKey;
    use std::collections::HashMap;

    #[derive(Clone)]
    struct MockLocalStore {
        tokens: TokenV3,
    }

    impl MockLocalStore {
        fn with_tokens(tokens: TokenV3) -> Self {
            Self { tokens }
        }
    }

    impl Default for MockLocalStore {
        fn default() -> Self {
            Self {
                tokens: TokenV3::new(Token {
                    mint: Some("mint_url".to_string()),
                    proofs: Proofs::empty(),
                }),
            }
        }
    }

    #[async_trait]
    impl LocalStore for MockLocalStore {
        async fn migrate(&self) {}

        async fn add_proofs(&self, _: &Proofs) -> Result<(), crate::error::MokshaWalletError> {
            Ok(())
        }

        async fn get_proofs(
            &self,
        ) -> Result<moksha_core::model::Proofs, crate::error::MokshaWalletError> {
            Ok(self.tokens.clone().proofs())
        }

        async fn delete_proofs(
            &self,
            _proofs: &Proofs,
        ) -> Result<(), crate::error::MokshaWalletError> {
            Ok(())
        }

        async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
            unimplemented!()
        }

        async fn add_keyset(&self, _keyset: &WalletKeyset) -> Result<(), MokshaWalletError> {
            unimplemented!()
        }
    }

    #[derive(Clone, Default)]
    struct MockClient {
        split_response: PostSplitResponse,
        post_mint_response: PostMintResponse,
        post_melt_response: PostMeltResponse,
    }

    impl MockClient {
        fn with_split_response(split_response: PostSplitResponse) -> Self {
            Self {
                split_response,
                ..Default::default()
            }
        }

        fn with_mint_response(post_mint_response: PostMintResponse) -> Self {
            Self {
                post_mint_response,
                ..Default::default()
            }
        }

        fn with_melt_response(post_melt_response: PostMeltResponse) -> Self {
            Self {
                post_melt_response,
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
            unimplemented!()
        }

        async fn get_mint_keysets(&self, _mint_url: &Url) -> Result<Keysets, MokshaWalletError> {
            unimplemented!()
        }

        async fn get_mint_payment_request(
            &self,
            _mint_url: &Url,
            _amount: u64,
        ) -> Result<PaymentRequest, MokshaWalletError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_mint_tokens() -> anyhow::Result<()> {
        let raw_response = read_fixture("post_mint_response_20.json")?;
        let mint_response = serde_json::from_str::<PostMintResponse>(&raw_response)?;

        let client = MockClient::with_mint_response(mint_response);
        let localstore = Box::<MockLocalStore>::default();
        let mint_url = "http://localhost:8080/";

        let mint_keyset = MintKeyset::new("superprivatesecretkey".to_string(), "".to_string());
        let wallet = Wallet::new(
            Box::new(client),
            mint_keyset.public_keys,
            Keysets::new(vec![mint_keyset.keyset_id]),
            localstore,
            Url::parse(mint_url).expect("invalid url"),
        );

        let result = wallet.mint_tokens(20.into(), "hash".to_string()).await?;
        assert_eq!(20, result.total_amount());
        assert_eq!(
            mint_url.to_owned(),
            result
                .tokens
                .get(0)
                .expect("Tokens is empty")
                .mint
                .as_ref()
                .expect("mint is empty")
                .clone()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_split() -> anyhow::Result<()> {
        let raw_response = read_fixture("post_split_response_24_40.json")?;
        let split_response = serde_json::from_str::<PostSplitResponse>(&raw_response)?;

        let client = MockClient::with_split_response(split_response);
        let localstore = Box::<MockLocalStore>::default();

        let mint_keyset = MintKeyset::new("mysecret".to_string(), "".to_string());
        let wallet = Wallet::new(
            Box::new(client),
            mint_keyset.public_keys,
            Keysets::new(vec![mint_keyset.keyset_id]),
            localstore,
            Url::parse("http://localhost:8080").expect("invalid url"),
        );

        let tokens = read_fixture("token_64.cashu")?.try_into()?;
        let result = wallet.split_tokens(&tokens, 20.into()).await?;
        assert_eq!(24, result.0.total_amount());
        assert_eq!(40, result.1.total_amount());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_balance() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let local_store = MockLocalStore::with_tokens(fixture.try_into()?);

        let wallet = Wallet::new(
            Box::<MockClient>::default(),
            HashMap::new(),
            Keysets::new(vec!["foo".to_string()]),
            Box::new(local_store),
            Url::parse("http://localhost:8080").expect("invalid url"),
        );

        let result = wallet.get_balance().await?;
        assert_eq!(60, result);
        Ok(())
    }

    #[tokio::test]
    async fn test_pay_invoice() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let local_store = MockLocalStore::with_tokens(fixture.try_into()?);

        let melt_response = read_fixture("post_melt_response_21.json")?; // 60 tokens (4,8,16,32)
        let mock_client = MockClient::with_melt_response(serde_json::from_str::<PostMeltResponse>(
            &melt_response,
        )?);

        let mint_keyset = MintKeyset::new("mysecret".to_string(), "".to_string());
        let wallet = Wallet::new(
            Box::new(mock_client),
            mint_keyset.public_keys,
            Keysets::new(vec![mint_keyset.keyset_id]),
            Box::new(local_store),
            Url::parse("http://localhost:8080").expect("invalid url"),
        );

        // 21 sats
        let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

        let result = wallet.pay_invoice(invoice).await?;
        assert!(result.paid);
        Ok(())
    }

    fn read_fixture(name: &str) -> anyhow::Result<String> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
        Ok(raw_token.trim().to_string())
    }
}
