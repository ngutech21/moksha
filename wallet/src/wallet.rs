use std::collections::HashMap;

use cashurs_core::{
    dhke::Dhke,
    model::{
        split_amount, BlindedMessage, BlindedSignature, Keysets, PaymentRequest, PostMeltResponse,
        Proof, Proofs, TokenV3, TotalAmount,
    },
};
use secp256k1::{PublicKey, SecretKey};

use crate::{client::Client, error::CashuWalletError, localstore::LocalStore};
use lightning_invoice::Invoice as LNInvoice;
use rand::{distributions::Alphanumeric, Rng};
use std::str::FromStr;

pub struct Wallet {
    client: Box<dyn Client>,
    mint_keys: HashMap<u64, PublicKey>, // FIXME use specific type
    keysets: Keysets,
    dhke: Dhke,
    localstore: Box<dyn LocalStore + Sync + Send>,
    mint_url: String,
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

impl Wallet {
    pub fn new(
        client: Box<dyn Client>,
        mint_keys: HashMap<u64, PublicKey>,
        keysets: Keysets,
        localstore: Box<dyn LocalStore + Sync + Send>,
        mint_url: String,
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

    pub async fn get_mint_payment_request(
        &self,
        amount: u64,
    ) -> Result<PaymentRequest, CashuWalletError> {
        self.client.get_mint_payment_request(amount).await
    }

    pub async fn get_balance(&self) -> Result<u64, CashuWalletError> {
        let total = self.localstore.get_proofs().await?.total_amount();
        Ok(total)
    }

    pub async fn receive_tokens(&self, tokens: &TokenV3) -> Result<(), CashuWalletError> {
        let total_amount = tokens.total_amount();
        let (_, redeemed_tokens) = self.split_tokens(tokens, total_amount).await?;
        self.localstore
            .add_proofs(&redeemed_tokens.proofs())
            .await?;
        Ok(())
    }

    pub async fn pay_invoice(&self, invoice: String) -> Result<PostMeltResponse, CashuWalletError> {
        let all_proofs = self.localstore.get_proofs().await?;

        let fees = self.client.post_checkfees(invoice.clone()).await?;
        let ln_amount = self.get_invoice_amount(&invoice)? + (fees.fee / 1000);

        if ln_amount > all_proofs.total_amount() {
            println!("Not enough tokens");
            return Err(CashuWalletError::NotEnoughTokens);
        }
        let selected_proofs = self.get_proofs_for_amount(ln_amount).await?;

        let total_proofs = if selected_proofs.total_amount() > ln_amount {
            let selected_tokens = TokenV3::from((self.mint_url.clone(), selected_proofs.clone()));
            let split_result = self.split_tokens(&selected_tokens, ln_amount).await?;

            self.localstore.delete_proofs(&selected_proofs).await?;
            self.localstore.add_proofs(&split_result.0.proofs()).await?;

            split_result.1.proofs()
        } else {
            selected_proofs
        };

        self.melt_token(invoice, ln_amount, &total_proofs).await
    }

    pub async fn split_tokens(
        &self,
        tokens: &TokenV3,
        splt_amount: u64,
    ) -> Result<(TokenV3, TokenV3), CashuWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount = total_token_amount - splt_amount;
        let first_secrets = self.create_secrets(&split_amount(first_amount));
        let first_outputs = self.create_blinded_messages(first_amount, &first_secrets)?;

        // ############################################################################

        let second_amount = splt_amount;
        let second_secrets = self.create_secrets(&split_amount(second_amount));
        let second_outputs = self.create_blinded_messages(second_amount, &second_secrets)?;

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        if tokens.total_amount() != total_outputs.total_amount() {
            return Err(CashuWalletError::InvalidProofs);
        }

        let split_result = self
            .client
            .post_split_tokens(splt_amount, tokens.proofs(), total_outputs)
            .await?;

        let first_tokens = TokenV3::from((
            self.mint_url.clone(),
            self.create_proofs_from_blinded_signatures(
                split_result.fst,
                first_secrets,
                first_outputs,
            )?,
        ));

        let second_tokens = TokenV3::from((
            self.mint_url.clone(),
            self.create_proofs_from_blinded_signatures(
                split_result.snd,
                second_secrets,
                second_outputs,
            )?,
        ));

        Ok((first_tokens, second_tokens))
    }

    pub async fn melt_token(
        &self,
        pr: String,
        _invoice_amount: u64,
        proofs: &Proofs,
    ) -> Result<PostMeltResponse, CashuWalletError> {
        //   let remaining = proofs.get_total_amount() - invoice_amount;
        // let secrets = self.create_secrets(&split_amount(remaining));
        // let outputs_full = self.create_blinded_messages(remaining, secrets.clone())?;
        // let outputs = get_blinded_msg(outputs_full.clone());

        let melt_response = self
            .client
            .post_melt_tokens(proofs.clone(), pr, vec![])
            .await?;

        self.localstore.delete_proofs(proofs).await?;

        // let change = melt_response.change.clone();

        // let change_proofs =
        //     self.create_proofs_from_blinded_signatures(change, secrets, outputs_full)?;

        // println!("change_proofs: {change_proofs:?}");

        // self.localstore.add_tokens(Tokens::new(Token {
        //     mint: Some(self.mint_url.clone()),
        //     proofs: change_proofs,
        // }))?;

        Ok(melt_response)
    }

    fn decode_invoice(&self, payment_request: &str) -> Result<LNInvoice, CashuWalletError> {
        LNInvoice::from_str(payment_request)
            .map_err(|err| CashuWalletError::DecodeInvoice(payment_request.to_owned(), err))
    }

    fn get_invoice_amount(&self, payment_request: &str) -> Result<u64, CashuWalletError> {
        let invoice = self.decode_invoice(payment_request)?;
        Ok(invoice
            .amount_milli_satoshis()
            .ok_or_else(|| CashuWalletError::InvalidInvoice(payment_request.to_owned()))?
            / 1000)
    }

    fn create_secrets(&self, split_amount: &[u64]) -> Vec<String> {
        (0..split_amount.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }

    pub async fn mint_tokens(
        &self,
        amount: u64,
        hash: String,
    ) -> Result<TokenV3, CashuWalletError> {
        let splited_amount = split_amount(amount);
        let secrets = self.create_secrets(&splited_amount);

        let blinded_messages = splited_amount
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
                hash,
                blinded_messages
                    .clone()
                    .into_iter()
                    .map(|(msg, _)| msg)
                    .collect::<Vec<BlindedMessage>>(),
            )
            .await?;

        // step 3: unblind signatures
        let keysets = &self.keysets.keysets;
        let current_keyset = keysets[keysets.len() - 1].clone();

        let private_keys = blinded_messages
            .clone()
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        let proofs = Proofs::new(
            post_mint_resp
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
                .collect::<Vec<Proof>>(),
        );

        let tokens = TokenV3::from((self.mint_url.clone(), proofs));
        self.localstore.add_proofs(&tokens.proofs()).await?;

        Ok(tokens)
    }

    fn create_blinded_messages(
        &self,
        amount: u64,
        secrets: &[String],
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, CashuWalletError> {
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
    ) -> Result<Proofs, CashuWalletError> {
        let keysets = &self.keysets.keysets;
        let current_keyset = keysets[keysets.len() - 1].clone();

        let private_keys = outputs
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        Ok(Proofs::new(
            signatures
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
                .collect::<Vec<Proof>>(),
        ))
    }

    pub async fn get_proofs_for_amount(&self, amount: u64) -> Result<Proofs, CashuWalletError> {
        let all_proofs = self.localstore.get_proofs().await?;

        if amount > all_proofs.total_amount() {
            return Err(CashuWalletError::NotEnoughTokens);
        }

        let mut all_proofs = all_proofs.proofs();
        all_proofs.sort_by(|a, b| a.amount.cmp(&b.amount));

        let mut selected_proofs = vec![];
        let mut selected_amount = 0;

        while selected_amount < amount {
            if all_proofs.is_empty() {
                break;
            }

            let proof = all_proofs.pop().expect("proofs is empty");
            selected_amount += proof.amount;
            selected_proofs.push(proof);
        }

        Ok(Proofs::new(selected_proofs))
    }
}

// FIXME implement for Vec<BlindedMessage, Secretkey>
fn get_blinded_msg(blinded_messages: Vec<(BlindedMessage, SecretKey)>) -> Vec<BlindedMessage> {
    blinded_messages
        .into_iter()
        .map(|(msg, _)| msg)
        .collect::<Vec<BlindedMessage>>()
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Wallet;
    use crate::{
        client::{Client, HttpClient},
        error::CashuWalletError,
        localstore::{LocalStore, WalletKeyset},
    };
    use async_trait::async_trait;
    use cashurs_core::model::{
        BlindedMessage, CheckFeesResponse, Keysets, PaymentRequest, PostMeltResponse,
        PostMintResponse, PostSplitResponse, Proofs, Token, TokenV3,
    };
    use secp256k1::PublicKey;
    use std::collections::HashMap;

    #[derive(Clone)]
    struct MockLocalStore {
        tokens: TokenV3,
    }

    impl MockLocalStore {
        fn new() -> Self {
            Self {
                tokens: TokenV3::new(Token {
                    mint: Some("mint_url".to_string()),
                    proofs: Proofs::empty(),
                }),
            }
        }

        fn with_tokens(tokens: TokenV3) -> Self {
            Self { tokens }
        }
    }

    impl Default for MockLocalStore {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl LocalStore for MockLocalStore {
        async fn migrate(&self) {}

        async fn add_proofs(&self, _: &Proofs) -> Result<(), crate::error::CashuWalletError> {
            unimplemented!()
        }

        async fn get_proofs(
            &self,
        ) -> Result<cashurs_core::model::Proofs, crate::error::CashuWalletError> {
            Ok(self.tokens.clone().proofs())
        }

        async fn delete_proofs(
            &self,
            _proofs: &Proofs,
        ) -> Result<(), crate::error::CashuWalletError> {
            unimplemented!()
        }

        async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, CashuWalletError> {
            unimplemented!()
        }

        async fn add_keyset(&self, _keyset: &WalletKeyset) -> Result<(), CashuWalletError> {
            unimplemented!()
        }
    }

    #[derive(Clone)]
    struct MockClient {}

    impl MockClient {
        fn new() -> Self {
            Self {}
        }
    }

    impl PartialEq for MockClient {
        fn eq(&self, _other: &Self) -> bool {
            true
        }
    }

    #[async_trait]
    impl Client for MockClient {
        async fn post_split_tokens(
            &self,
            _amount: u64,
            _proofs: Proofs,
            _output: Vec<BlindedMessage>,
        ) -> Result<PostSplitResponse, CashuWalletError> {
            Ok(PostSplitResponse {
                fst: vec![],
                snd: vec![],
            })
        }

        async fn post_mint_payment_request(
            &self,
            _hash: String,
            _blinded_messages: Vec<BlindedMessage>,
        ) -> Result<PostMintResponse, CashuWalletError> {
            unimplemented!()
        }

        async fn post_melt_tokens(
            &self,
            _proofs: Proofs,
            _pr: String,
            _outputs: Vec<BlindedMessage>,
        ) -> Result<PostMeltResponse, CashuWalletError> {
            unimplemented!()
        }

        async fn post_checkfees(&self, _pr: String) -> Result<CheckFeesResponse, CashuWalletError> {
            unimplemented!()
        }

        async fn get_mint_keys(&self) -> Result<HashMap<u64, PublicKey>, CashuWalletError> {
            unimplemented!()
        }

        async fn get_mint_keysets(&self) -> Result<Keysets, CashuWalletError> {
            unimplemented!()
        }

        async fn get_mint_payment_request(
            &self,
            _amount: u64,
        ) -> Result<PaymentRequest, CashuWalletError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_create_secrets() {
        let client = HttpClient::new("http://localhost:8080".to_string());
        let localstore = Box::new(MockLocalStore::new());
        let wallet = Wallet::new(
            Box::new(client),
            HashMap::new(),
            Keysets { keysets: vec![] },
            localstore,
            "mint_url".to_string(),
        );

        let amounts = vec![1, 2, 3, 4, 5, 6, 7];
        let secrets = wallet.create_secrets(&amounts);

        assert!(secrets.len() == amounts.len());
    }

    #[tokio::test]
    async fn test_split() -> anyhow::Result<()> {
        let client = MockClient::new();
        let localstore = Box::new(MockLocalStore::new());

        let wallet = Wallet::new(
            Box::new(client),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
            localstore,
            "mint_url".to_string(),
        );

        // read file
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/token_64.cashu"))?;
        let tokens = TokenV3::deserialize(raw_token.trim().to_string())?;

        let result = wallet.split_tokens(&tokens, 20).await?;
        // assert_eq!(20, result.0.total_amount());
        // assert_eq!(44, result.1.total_amount());
        println!("{result:?}");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_proofs_for_amount_empty() -> anyhow::Result<()> {
        let local_store = MockLocalStore::new();

        let wallet = Wallet::new(
            Box::new(MockClient::new()),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
            Box::new(local_store),
            "mint_url".to_string(),
        );

        let result = wallet.get_proofs_for_amount(10).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_proofs_for_amount_valid() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let local_store = MockLocalStore::with_tokens(fixture);

        let wallet = Wallet::new(
            Box::new(MockClient::new()),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
            Box::new(local_store),
            "mint_url".to_string(),
        );

        let result = wallet.get_proofs_for_amount(10).await?;
        println!("{result:?}");

        assert_eq!(32, result.total_amount());
        assert_eq!(1, result.len());
        Ok(())
    }

    fn read_fixture(name: &str) -> anyhow::Result<TokenV3> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
        Ok(TokenV3::deserialize(raw_token.trim().to_string())?)
    }
}
