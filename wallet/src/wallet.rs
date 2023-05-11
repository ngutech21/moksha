use std::collections::HashMap;

use cashurs_core::{
    dhke::Dhke,
    model::{
        split_amount, BlindedMessage, BlindedSignature, Keysets, PostMeltResponse, Proof, Proofs,
        Token, Tokens, TotalAmount,
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
    localstore: Box<dyn LocalStore>,
    mint_url: String,
}

impl Wallet {
    pub fn new(
        client: Box<dyn Client>,
        mint_keys: HashMap<u64, PublicKey>,
        keysets: Keysets,
        localstore: Box<dyn LocalStore>,
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

    pub fn get_balance(&self) -> Result<u64, CashuWalletError> {
        let total = self.localstore.get_tokens()?.total_amount();
        Ok(total)
    }

    pub async fn split_tokens(
        &self,
        tokens: Tokens,
        splt_amount: u64,
    ) -> Result<(Tokens, Tokens), CashuWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount = total_token_amount - splt_amount;
        let first_secrets = self.create_secrets(&split_amount(first_amount));
        let first_outputs = self.create_blinded_messages(first_amount, first_secrets.clone())?;

        // ############################################################################

        let second_amount = splt_amount;
        let second_secrets = self.create_secrets(&split_amount(second_amount));
        let second_outputs = self.create_blinded_messages(second_amount, second_secrets.clone())?;

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        if tokens.total_amount() != total_outputs.total_amount() {
            return Err(CashuWalletError::InvalidProofs);
        }

        let split_result = self
            .client
            .post_split_tokens(splt_amount, tokens.get_proofs(), total_outputs)
            .await?;

        let first_tokens = Tokens::from((
            self.mint_url.clone(),
            self.create_proofs_from_blinded_signatures(
                split_result.fst,
                first_secrets,
                first_outputs,
            )?,
        ));

        let second_tokens = Tokens::from((
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
        proofs: Proofs,
    ) -> Result<PostMeltResponse, CashuWalletError> {
        let _fees = self.client.post_checkfees(pr.clone()).await.unwrap();
        // TODO get tokens for fee amount

        let invoice_amount = self.get_invoice_amount(&pr)?;

        let remaining = proofs.get_total_amount() - invoice_amount;

        let secrets = self.create_secrets(&split_amount(remaining));
        let outputs_full = self.create_blinded_messages(remaining, secrets.clone())?;
        let outputs = get_blinded_msg(outputs_full.clone());

        let melt_response = self
            .client
            .post_melt_tokens(proofs.clone(), pr, outputs)
            .await?;

        self.localstore.delete_tokens(Tokens::new(Token {
            mint: Some(self.mint_url.clone()),
            proofs,
        }))?;

        let change = melt_response.change.clone();

        let change_proofs =
            self.create_proofs_from_blinded_signatures(change, secrets, outputs_full)?;

        println!("change_proofs: {:?}", change_proofs);

        self.localstore.add_tokens(Tokens::new(Token {
            mint: Some(self.mint_url.clone()),
            proofs: change_proofs,
        }))?;

        Ok(melt_response)
    }

    pub fn decode_invoice(&self, payment_request: &str) -> Result<LNInvoice, CashuWalletError> {
        LNInvoice::from_str(payment_request)
            .map_err(|err| CashuWalletError::DecodeInvoice(payment_request.to_owned(), err))
    }

    pub fn get_invoice_amount(&self, payment_request: &str) -> Result<u64, CashuWalletError> {
        let invoice = self.decode_invoice(payment_request)?;
        Ok(invoice.amount_milli_satoshis().unwrap() / 1000) // FIXME unwrap
    }

    pub fn create_secrets(&self, split_amount: &Vec<u64>) -> Vec<String> {
        (0..split_amount.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }

    pub async fn mint_tokens(&self, amount: u64, hash: String) -> Result<Tokens, CashuWalletError> {
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

        let tokens = Tokens::from((self.mint_url.clone(), proofs));
        self.localstore.add_tokens(tokens.clone())?;

        Ok(tokens)
    }

    pub fn create_blinded_messages(
        &self,
        amount: u64,
        secrets: Vec<String>,
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, CashuWalletError> {
        let split_amount = split_amount(amount);

        Ok(split_amount
            .into_iter()
            .zip(secrets)
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
                (BlindedMessage { amount, b_ }, alice_secret_key)
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>())
    }

    pub fn create_proofs_from_blinded_signatures(
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

    pub fn get_proofs_for_amount(&self, amount: u64) -> Result<Proofs, CashuWalletError> {
        let all_tokens = self.localstore.get_tokens()?;

        if amount > all_tokens.total_amount() {
            return Err(CashuWalletError::NotEnoughTokens);
        }

        let mut all_proofs = all_tokens
            .tokens
            .iter()
            .flat_map(|token| token.proofs.get_proofs())
            .collect::<Vec<Proof>>();
        all_proofs.sort_by(|a, b| a.amount.partial_cmp(&b.amount).unwrap());

        let mut selected_proofs = vec![];
        let mut selected_amount = 0;

        while selected_amount < amount {
            if all_proofs.is_empty() {
                break;
            }

            let proof = all_proofs.pop().unwrap();
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
        client::{HttpClient, MockClient},
        localstore::MockLocalStore,
    };
    use cashurs_core::model::{Keysets, PostSplitResponse, Proofs, Token, Tokens};
    use std::collections::HashMap;

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
        let mut client = MockClient::new();

        client.expect_post_split_tokens().returning(|_, _, _| {
            Ok(PostSplitResponse {
                fst: vec![],
                snd: vec![],
            })
        });

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
        let tokens = Tokens::deserialize(raw_token.trim().to_string())?;

        let result = wallet.split_tokens(tokens, 20).await;
        // TODO add asserts for test
        println!("{:?}", result);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_proofs_for_amount_empty() -> anyhow::Result<()> {
        let mut local_store = MockLocalStore::new();
        local_store.expect_get_tokens().returning(|| {
            Ok(Tokens::new(Token {
                mint: Some("mint_url".to_string()),
                proofs: Proofs::empty(),
            }))
        });

        let wallet = Wallet::new(
            Box::new(MockClient::new()),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
            Box::new(local_store),
            "mint_url".to_string(),
        );

        let result = wallet.get_proofs_for_amount(10);

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_proofs_for_amount_valid() -> anyhow::Result<()> {
        let mut local_store = MockLocalStore::new();

        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)

        local_store.expect_get_tokens().returning(move || {
            Ok(Tokens::new(Token {
                mint: Some("mint_url".to_string()),
                proofs: fixture.get_proofs(),
            }))
        });

        let wallet = Wallet::new(
            Box::new(MockClient::new()),
            HashMap::new(),
            Keysets {
                keysets: vec!["foo".to_string()],
            },
            Box::new(local_store),
            "mint_url".to_string(),
        );

        let result = wallet.get_proofs_for_amount(10)?;
        println!("{:?}", result);

        assert_eq!(32, result.get_total_amount());
        assert_eq!(1, result.len());
        Ok(())
    }

    fn read_fixture(name: &str) -> anyhow::Result<Tokens> {
        let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
        Ok(Tokens::deserialize(raw_token.trim().to_string())?)
    }
}
