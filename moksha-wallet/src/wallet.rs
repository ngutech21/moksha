use moksha_core::{
    amount::Amount,
    blind::{BlindedMessage, BlindedSignature, TotalAmount},
    dhke::Dhke,
    keyset::V1Keyset,
    primitives::{
        CurrencyUnit, KeyResponse, MintInfoResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltBtcOnchainResponse, PostMeltQuoteBolt11Response, PostMeltQuoteBtcOnchainResponse,
        PostMintQuoteBolt11Response, PostMintQuoteBtcOnchainResponse,
    },
    proof::{Proof, Proofs},
    token::TokenV3,
};

use secp256k1::SecretKey;
use url::Url;

use crate::{
    client::CashuClient,
    error::MokshaWalletError,
    http::CrossPlatformHttpClient,
    localstore::{LocalStore, WalletKeyset},
    secret::{self, DeterministicSecret},
};
use lightning_invoice::Bolt11Invoice as LNInvoice;
use std::str::FromStr;

#[derive(Clone)]
pub struct Wallet<L, C>
where
    L: LocalStore,
    C: CashuClient,
{
    client: C,
    keyset_id: V1Keyset,
    keyset: KeyResponse,
    dhke: Dhke,
    localstore: L,
    mint_url: Url,
    secret: DeterministicSecret,
}

pub struct WalletBuilder<L, C: CashuClient = CrossPlatformHttpClient>
where
    L: LocalStore,
    C: CashuClient + Default,
{
    client: Option<C>,
    localstore: Option<L>,
    mint_url: Option<Url>,
}

impl<L, C> WalletBuilder<L, C>
where
    L: LocalStore,
    C: CashuClient + Default,
{
    fn new() -> Self {
        Self {
            client: Some(C::default()),
            localstore: None,
            mint_url: None,
        }
    }

    pub fn with_client(mut self, client: C) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_localstore(mut self, localstore: L) -> Self {
        self.localstore = Some(localstore);
        self
    }

    pub fn with_mint_url(mut self, mint_url: Url) -> Self {
        self.mint_url = Some(mint_url);
        self
    }

    pub async fn build(self) -> Result<Wallet<L, C>, MokshaWalletError> {
        let client = self.client.unwrap_or_default();
        let localstore = self.localstore.expect("localstore is required");
        let mint_url = self.mint_url.expect("mint_url is required");

        if !client.is_v1_supported(&mint_url).await? {
            return Err(MokshaWalletError::UnsupportedApiVersion);
        }

        let mut tx = localstore.begin_tx().await?;

        let seed_words = localstore.get_seed(&mut tx).await?;
        if seed_words.is_none() {
            let seed = DeterministicSecret::generate_random_seed_words()?;
            localstore.add_seed(&mut tx, &seed).await?;
        }

        let mint_keysets = client.get_keysets(&mint_url).await?;

        for m in mint_keysets.keysets.iter() {
            let public_keys = client
                .get_keys_by_id(&mint_url, m.id.clone())
                .await?
                .keysets
                .into_iter()
                .find(|k| k.id == m.id)
                .expect("no valid keyset found")
                .keys
                .clone();

            let wallet_keyset = WalletKeyset::new(&m.id, &mint_url, &m.unit, 0, public_keys, true);
            localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
        }
        let seed = localstore.get_seed(&mut tx).await?.expect("seed not found");
        tx.commit().await?;

        // FIXME store all keysets
        let keys = client.get_keys(&mint_url).await?;

        let key_response = keys
            .keysets
            .iter()
            .find(|k| k.id.starts_with("00"))
            .expect("no valid keyset found");

        let mks = mint_keysets
            .keysets
            .iter()
            .find(|k| k.id.starts_with("00"))
            .expect("no valid keyset found");

        Ok(Wallet::new(
            client as C,
            mks.clone(),
            key_response.clone(),
            localstore,
            mint_url,
            DeterministicSecret::from_seed_words(&seed)?,
        ))
    }
}

impl<L, C> Default for WalletBuilder<L, C>
where
    C: CashuClient + Default,
    L: LocalStore,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<L, C> Wallet<L, C>
where
    C: CashuClient + Default,
    L: LocalStore,
{
    fn new(
        client: C,
        mint_keys: V1Keyset,
        key_response: KeyResponse,
        localstore: L,
        mint_url: Url,
        secret: DeterministicSecret,
    ) -> Self {
        Self {
            client,
            keyset_id: mint_keys,
            keyset: key_response,
            dhke: Dhke::new(),
            localstore,
            mint_url,
            secret,
        }
    }

    pub fn builder() -> WalletBuilder<L, C> {
        WalletBuilder::default()
    }

    pub async fn create_quote_bolt11(
        &self,
        amount: u64,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_mint_quote_bolt11(&self.mint_url, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn create_quote_onchain(
        &self,
        amount: u64,
    ) -> Result<PostMintQuoteBtcOnchainResponse, MokshaWalletError> {
        self.client
            .post_mint_quote_onchain(&self.mint_url, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn is_quote_paid(
        &self,
        payment_method: &PaymentMethod,
        quote: String,
    ) -> Result<bool, MokshaWalletError> {
        Ok(match payment_method {
            PaymentMethod::Bolt11 => {
                self.client
                    .get_mint_quote_bolt11(&self.mint_url, quote)
                    .await?
                    .paid
            }

            PaymentMethod::BtcOnchain => {
                self.client
                    .get_mint_quote_onchain(&self.mint_url, quote)
                    .await?
                    .paid
            }
        })
    }

    pub async fn is_onchain_paid(&self, quote: String) -> Result<bool, MokshaWalletError> {
        Ok(self
            .client
            .get_melt_quote_onchain(&self.mint_url, quote)
            .await?
            .paid)
    }

    pub async fn is_onchain_tx_paid(&self, txid: String) -> Result<bool, MokshaWalletError> {
        Ok(self
            .client
            .get_melt_onchain(&self.mint_url, txid)
            .await?
            .paid)
    }

    pub async fn get_balance(&self) -> Result<u64, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        Ok(self.localstore.get_proofs(&mut tx).await?.total_amount())
    }

    pub async fn send_tokens(&self, amount: u64) -> Result<TokenV3, MokshaWalletError> {
        let balance = self.get_balance().await?;
        if amount > balance {
            return Err(MokshaWalletError::NotEnoughTokens);
        }

        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self.localstore.get_proofs(&mut tx).await?;
        let selected_proofs = all_proofs.proofs_for_amount(amount)?;
        let selected_tokens = (self.mint_url.to_owned(), selected_proofs.clone()).into();

        let (remaining_tokens, result) = self.split_tokens(&selected_tokens, amount.into()).await?;

        self.localstore
            .delete_proofs(&mut tx, &selected_proofs)
            .await?;
        self.localstore
            .add_proofs(&mut tx, &remaining_tokens.proofs())
            .await?;
        tx.commit().await?;

        Ok(result)
    }

    pub async fn receive_tokens(&self, tokens: &TokenV3) -> Result<(), MokshaWalletError> {
        let total_amount = tokens.total_amount();
        let (_, redeemed_tokens) = self.split_tokens(tokens, total_amount.into()).await?;
        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .add_proofs(&mut tx, &redeemed_tokens.proofs())
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_mint_quote(
        &self,
        amount: Amount,
        currency: CurrencyUnit,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_mint_quote_bolt11(&self.mint_url, amount.0, currency)
            .await
    }

    pub async fn get_melt_quote_bolt11(
        &self,
        invoice: String,
        currency: CurrencyUnit,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_melt_quote_bolt11(&self.mint_url, invoice.clone(), currency)
            .await
    }

    pub async fn pay_invoice(
        &self,
        melt_quote: &PostMeltQuoteBolt11Response,
        invoice: String,
    ) -> Result<(PostMeltBolt11Response, u64), MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self.localstore.get_proofs(&mut tx).await?;

        let ln_amount = Self::get_invoice_amount(&invoice)? + melt_quote.fee_reserve;

        if ln_amount > all_proofs.total_amount() {
            return Err(MokshaWalletError::NotEnoughTokens);
        }
        let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

        let total_proofs = {
            let selected_tokens = (self.mint_url.to_owned(), selected_proofs.clone()).into();
            let split_result = self
                .split_tokens(&selected_tokens, ln_amount.into())
                .await?;

            self.localstore
                .delete_proofs(&mut tx, &selected_proofs)
                .await?;
            self.localstore
                .add_proofs(&mut tx, &split_result.0.proofs())
                .await?;
            tx.commit().await?;

            split_result.1.proofs()
        };

        let fee_blind =
            BlindedMessage::blank(melt_quote.fee_reserve.into(), self.keyset_id.clone().id)?;

        let msgs = fee_blind
            .iter()
            .map(|(msg, _, _)| msg.clone())
            .collect::<Vec<BlindedMessage>>();

        let secrets = fee_blind
            .iter()
            .map(|(_, _, secret)| secret.clone())
            .collect::<Vec<String>>();

        let outputs = fee_blind
            .iter()
            .map(|(msg, secret, _)| (msg.clone(), secret.to_owned()))
            .collect::<Vec<(BlindedMessage, SecretKey)>>();

        let mut tx = self.localstore.begin_tx().await?;
        match self
            .melt_token(melt_quote.to_owned().quote, ln_amount, &total_proofs, msgs)
            .await
        {
            Ok(response) => {
                if !response.paid {
                    self.localstore.add_proofs(&mut tx, &total_proofs).await?;
                }
                let change_proofs = self.create_proofs_from_blinded_signatures(
                    response.clone().change,
                    secrets,
                    outputs,
                )?;
                self.localstore.add_proofs(&mut tx, &change_proofs).await?;
                tx.commit().await?;

                Ok((response, change_proofs.total_amount()))
            }
            Err(e) => {
                self.localstore.add_proofs(&mut tx, &total_proofs).await?;
                tx.commit().await?;
                Err(e)
            }
        }
    }

    pub async fn get_melt_quote_btconchain(
        &self,
        address: String,
        amount: u64,
    ) -> Result<Vec<PostMeltQuoteBtcOnchainResponse>, MokshaWalletError> {
        self.client
            .post_melt_quote_onchain(&self.mint_url, address, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn pay_onchain(
        &self,

        melt_quote: &PostMeltQuoteBtcOnchainResponse,
    ) -> Result<PostMeltBtcOnchainResponse, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self.localstore.get_proofs(&mut tx).await?;

        let ln_amount = melt_quote.amount + melt_quote.fee;

        if ln_amount > all_proofs.total_amount() {
            return Err(MokshaWalletError::NotEnoughTokens);
        }
        let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

        let total_proofs = {
            let selected_tokens = (self.mint_url.to_owned(), selected_proofs.clone()).into();
            let split_result = self
                .split_tokens(&selected_tokens, ln_amount.into())
                .await?;

            self.localstore
                .delete_proofs(&mut tx, &selected_proofs)
                .await?;
            self.localstore
                .add_proofs(&mut tx, &split_result.0.proofs())
                .await?;

            split_result.1.proofs()
        };

        let melt_response = self
            .client
            .post_melt_onchain(
                &self.mint_url,
                total_proofs.clone(),
                melt_quote.quote.clone(),
            )
            .await?;

        if melt_response.paid {
            self.localstore
                .delete_proofs(&mut tx, &total_proofs)
                .await?;
        }
        tx.commit().await?;
        Ok(melt_response)
    }

    async fn create_secrets(
        &self,
        amount: u32,
    ) -> Result<Vec<(String, SecretKey)>, MokshaWalletError> {
        let keyset_id = self.keyset_id.clone().id;
        let keyset_id_int = secret::convert_hex_to_int(&keyset_id).unwrap(); // FIXME

        let mut tx = self.localstore.begin_tx().await?;
        let all_keysets = self.localstore.get_keysets(&mut tx).await?;
        let keyset = all_keysets
            .iter()
            .find(|k| k.keyset_id == keyset_id)
            .expect("keyset not found");

        let start_index = (keyset.last_index + 1) as u32;
        let secret_range = self
            .secret
            .derive_range(keyset_id_int, start_index, amount)?;

        self.localstore
            .update_keyset_last_index(
                &mut tx,
                &WalletKeyset {
                    last_index: (start_index + amount) as u64,
                    ..keyset.clone()
                },
            )
            .await?;
        tx.commit().await?;
        Ok(secret_range)
    }

    pub async fn split_tokens(
        &self,
        tokens: &TokenV3,
        splt_amount: Amount,
    ) -> Result<(TokenV3, TokenV3), MokshaWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount: Amount = (total_token_amount - splt_amount.0).into();
        let first_secrets = self
            .create_secrets(first_amount.split().len() as u32)
            .await?;
        let first_outputs = self.create_blinded_messages(first_amount, &first_secrets)?;

        // ############################################################################

        let second_amount = splt_amount.clone();
        let second_secrets = self
            .create_secrets(second_amount.split().len() as u32)
            .await?;
        let second_outputs = self.create_blinded_messages(second_amount, &second_secrets)?;

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        if tokens.total_amount() != total_outputs.total_amount() {
            return Err(MokshaWalletError::InvalidProofs);
        }

        let split_result = self
            .client
            .post_swap(&self.mint_url, tokens.proofs(), total_outputs)
            .await?;

        if split_result.signatures.is_empty() {
            return Ok((TokenV3::empty(), TokenV3::empty()));
        }

        let len_first = first_secrets.len();
        let secrets = [first_secrets, second_secrets].concat();
        let outputs = [first_outputs, second_outputs].concat();

        let secrets = secrets.into_iter().map(|(s, _)| s).collect::<Vec<String>>();

        let proofs = self
            .create_proofs_from_blinded_signatures(split_result.signatures, secrets, outputs)?
            .proofs();

        let first_tokens = (
            self.mint_url.to_owned(),
            proofs[0..len_first].to_vec().into(),
        )
            .into();
        let second_tokens = (
            self.mint_url.to_owned(),
            proofs[len_first..proofs.len()].to_vec().into(),
        )
            .into();

        Ok((first_tokens, second_tokens))
    }

    pub async fn get_mint_info(&self) -> Result<MintInfoResponse, MokshaWalletError> {
        self.client.get_info(&self.mint_url).await
    }

    async fn melt_token(
        &self,
        quote_id: String,
        _invoice_amount: u64,
        proofs: &Proofs,
        fee_blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMeltBolt11Response, MokshaWalletError> {
        let melt_response = self
            .client
            .post_melt_bolt11(
                &self.mint_url,
                proofs.clone(),
                quote_id,
                fee_blinded_messages,
            )
            .await?;

        if melt_response.paid {
            let mut tx = self.localstore.begin_tx().await?;
            self.localstore.delete_proofs(&mut tx, proofs).await?;
            tx.commit().await?;
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
        payment_method: &PaymentMethod,
        amount: Amount,
        quote_id: String,
    ) -> Result<TokenV3, MokshaWalletError> {
        let split_amount = amount.split();

        // FIXME cleanup code
        let keyset_id = self.keyset_id.clone().id;
        let keyset_id_int = secret::convert_hex_to_int(&keyset_id).unwrap(); // FIXME

        let mut tx = self.localstore.begin_tx().await?;
        let all_keysets = self.localstore.get_keysets(&mut tx).await?;
        let keyset = all_keysets
            .iter()
            .find(|k| k.keyset_id == keyset_id)
            .expect("keyset not found");

        let start_index = (keyset.last_index + 1) as u32;
        let secret_range =
            self.secret
                .derive_range(keyset_id_int, start_index, split_amount.len() as u32)?;

        self.localstore
            .update_keyset_last_index(
                &mut tx,
                &WalletKeyset {
                    last_index: (start_index + split_amount.len() as u32) as u64,
                    ..keyset.clone()
                },
            )
            .await?;
        tx.commit().await?;

        let blinded_messages = split_amount
            .into_iter()
            .zip(secret_range)
            .map(|(amount, (secret, blinding_factor))| {
                let (b_, alice_secret_key) = self
                    .dhke
                    .step1_alice(&secret, Some(&blinding_factor.secret_bytes()))
                    .unwrap(); // FIXME
                (
                    BlindedMessage {
                        amount,
                        b_,
                        id: self.keyset_id.clone().id,
                    },
                    alice_secret_key,
                    secret,
                )
            })
            .collect::<Vec<(BlindedMessage, SecretKey, String)>>();

        let signatures = match payment_method {
            PaymentMethod::Bolt11 => {
                let post_mint_resp = self
                    .client
                    .post_mint_bolt11(
                        &self.mint_url,
                        quote_id,
                        blinded_messages
                            .clone()
                            .into_iter()
                            .map(|(msg, _, _)| msg)
                            .collect::<Vec<BlindedMessage>>(),
                    )
                    .await?;
                post_mint_resp.signatures
            }
            PaymentMethod::BtcOnchain => {
                let post_mint_resp = self
                    .client
                    .post_mint_onchain(
                        &self.mint_url,
                        quote_id,
                        blinded_messages
                            .clone()
                            .into_iter()
                            .map(|(msg, _, _)| msg)
                            .collect::<Vec<BlindedMessage>>(),
                    )
                    .await?;
                post_mint_resp.signatures
            }
        };

        // step 3: unblind signatures
        let current_keyset_id = self.keyset_id.clone().id; // FIXME

        let proofs = signatures
            .iter()
            .zip(blinded_messages)
            .map(|(p, (_, priv_key, secret))| {
                let key = self
                    .keyset
                    .keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset_id.clone())
            })
            .collect::<Vec<Proof>>()
            .into();

        let tokens: TokenV3 = (self.mint_url.to_owned(), proofs).into();
        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .add_proofs(&mut tx, &tokens.proofs())
            .await?;
        tx.commit().await?;

        Ok(tokens)
    }

    // FIXME implement for Amount
    fn create_blinded_messages(
        &self,
        amount: Amount,
        secrets_factors: &Vec<(String, SecretKey)>,
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, MokshaWalletError> {
        let split_amount = amount.split();

        Ok(split_amount
            .into_iter()
            .zip(secrets_factors)
            .map(|(amount, (secret, blinding_factor))| {
                let (b_, alice_secret_key) = self
                    .dhke
                    .step1_alice(secret, Some(&blinding_factor.secret_bytes()))
                    .unwrap(); // FIXME
                (
                    BlindedMessage {
                        amount,
                        b_,
                        id: self.keyset_id.clone().id,
                    },
                    alice_secret_key,
                )
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>())
    }

    fn create_proofs_from_blinded_signatures(
        &self,
        signatures: Vec<BlindedSignature>,
        secrets: Vec<String>,
        outputs: Vec<(BlindedMessage, SecretKey)>,
    ) -> Result<Proofs, MokshaWalletError> {
        let current_keyset_id = self.keyset_id.clone().id; // FIXME

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
                    .keyset
                    .keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset_id.clone())
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
    use crate::client::MockCashuClient;
    use crate::localstore::sqlite::SqliteLocalStore;
    use crate::localstore::LocalStore;
    use crate::wallet::WalletBuilder;

    use moksha_core::fixture::{read_fixture, read_fixture_as};
    use moksha_core::keyset::{MintKeyset, V1Keysets};
    use moksha_core::primitives::{
        CurrencyUnit, KeyResponse, KeysResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltQuoteBolt11Response, PostMintBolt11Response, PostSwapResponse,
    };

    use moksha_core::token::TokenV3;
    use url::Url;

    fn create_mock() -> MockCashuClient {
        let keys = MintKeyset::new("mykey", "");
        let key_response = KeyResponse {
            keys: keys.public_keys.clone(),
            id: keys.keyset_id.clone(),
            unit: CurrencyUnit::Sat,
        };
        let keys_response = KeysResponse::new(key_response.clone());
        let keys_by_id_response = keys_response.clone();
        let keysets = V1Keysets::new(keys.keyset_id, CurrencyUnit::Sat, true);

        let mut client = MockCashuClient::default();
        client
            .expect_get_keys()
            .returning(move |_| Ok(keys_response.clone()));
        client
            .expect_get_keysets()
            .returning(move |_| Ok(keysets.clone()));
        client
            .expect_get_keys_by_id()
            .returning(move |_, _| Ok(keys_by_id_response.clone()));
        client.expect_is_v1_supported().returning(move |_| Ok(true));
        client
    }

    #[tokio::test]
    async fn test_mint_tokens() -> anyhow::Result<()> {
        let mint_response =
            read_fixture_as::<PostMintBolt11Response>("post_mint_response_20.json")?;

        let mut client = create_mock();
        client
            .expect_post_mint_bolt11()
            .returning(move |_, _, _| Ok(mint_response.clone()));

        let localstore = SqliteLocalStore::with_in_memory().await?;
        let mint_url = Url::parse("http://localhost:8080/").expect("invalid url");

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .with_mint_url(mint_url.clone())
            .build()
            .await?;

        let result = wallet
            .mint_tokens(&PaymentMethod::Bolt11, 20.into(), "hash".to_string())
            .await?;
        assert_eq!(20, result.total_amount());
        result.tokens.into_iter().for_each(|t| {
            assert_eq!(mint_url, t.mint.expect("mint is empty"));
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_split() -> anyhow::Result<()> {
        let split_response = read_fixture_as::<PostSwapResponse>("post_split_response_24_40.json")?;
        let mut client = create_mock();
        client
            .expect_post_swap()
            .returning(move |_, _, _| Ok(split_response.clone()));
        let localstore = SqliteLocalStore::with_in_memory().await?;

        let mint_url = Url::parse("http://localhost:8080/").expect("invalid url");
        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .with_mint_url(mint_url)
            .build()
            .await?;

        let tokens = read_fixture("token_64.cashu")?.try_into()?;
        let result = wallet.split_tokens(&tokens, 20.into()).await?;
        assert_eq!(24, result.0.total_amount());
        assert_eq!(40, result.1.total_amount());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_balance() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let fixture: TokenV3 = fixture.try_into()?;
        let local_store = SqliteLocalStore::with_in_memory().await?;
        let mut tx = local_store.begin_tx().await?;
        local_store.add_proofs(&mut tx, &fixture.proofs()).await?;
        tx.commit().await?;

        let mint_url = Url::parse("http://localhost:8080/").expect("invalid url");
        let wallet = WalletBuilder::new()
            .with_client(create_mock())
            .with_localstore(local_store)
            .with_mint_url(mint_url)
            .build()
            .await?;

        let result = wallet.get_balance().await?;
        assert_eq!(60, result);
        Ok(())
    }
    // FIXME

    #[tokio::test]
    async fn test_pay_invoice() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)

        let local_store = SqliteLocalStore::with_in_memory().await?;
        let fixture: TokenV3 = fixture.try_into()?;
        let mut tx = local_store.begin_tx().await?;
        local_store.add_proofs(&mut tx, &fixture.proofs()).await?;
        tx.commit().await?;

        let melt_response =
            read_fixture_as::<PostMeltBolt11Response>("post_melt_response_21.json")?; // 60 tokens (4,8,16,32)
        let mut mock_client = create_mock();
        mock_client
            .expect_post_melt_bolt11()
            .returning(move |_, _, _, _| Ok(melt_response.clone()));

        let quote_response =
            read_fixture_as::<PostMeltQuoteBolt11Response>("post_melt_quote_response.json")?;
        mock_client
            .expect_post_melt_quote_bolt11()
            .returning(move |_, _, _| Ok(quote_response.clone()));

        let swap_response = read_fixture_as::<PostSwapResponse>("post_split_response_24_40.json")?;
        mock_client
            .expect_post_swap()
            .returning(move |_, _, _| Ok(swap_response.clone()));

        let mint_url = Url::parse("http://localhost:8080/").expect("invalid url");
        let wallet = WalletBuilder::new()
            .with_client(mock_client)
            .with_localstore(local_store)
            .with_mint_url(mint_url)
            .build()
            .await?;

        // 21 sats
        let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

        let quote = wallet
            .get_melt_quote_bolt11(invoice.clone(), CurrencyUnit::Sat)
            .await?;

        let result = wallet.pay_invoice(&quote, invoice).await?;
        assert!(result.0.paid);
        Ok(())
    }

    #[tokio::test]
    async fn test_pay_invoice_can_not_melt() -> anyhow::Result<()> {
        let fixture = read_fixture("token_64.cashu")?; // 60 tokens (4,8,16,32)
        let tokens: TokenV3 = fixture.try_into()?;

        let localstore = SqliteLocalStore::with_in_memory()
            .await
            .expect("Could not create localstore");

        let mut tx = localstore.begin_tx().await?;
        localstore.add_proofs(&mut tx, &tokens.proofs()).await?;
        assert_eq!(64, localstore.get_proofs(&mut tx).await?.total_amount());
        tx.commit().await?;

        let melt_response =
            read_fixture_as::<PostMeltBolt11Response>("post_melt_response_not_paid.json")?;

        let mut mock_client = create_mock();
        mock_client
            .expect_post_melt_bolt11()
            .returning(move |_, _, _, _| Ok(melt_response.clone()));

        let quote_response =
            read_fixture_as::<PostMeltQuoteBolt11Response>("post_melt_quote_response.json")?;
        mock_client
            .expect_post_melt_quote_bolt11()
            .returning(move |_, _, _| Ok(quote_response.clone()));
        let swap_response = read_fixture_as::<PostSwapResponse>("post_split_response_24_40.json")?;
        mock_client
            .expect_post_swap()
            .returning(move |_, _, _| Ok(swap_response.clone()));

        let wallet = WalletBuilder::default()
            .with_client(mock_client)
            .with_localstore(localstore.clone())
            .with_mint_url(Url::parse("http://localhost:8080").expect("invalid url"))
            .build()
            .await?;

        // 21 sats
        let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

        let quote = wallet
            .get_melt_quote_bolt11(invoice.clone(), CurrencyUnit::Sat)
            .await?;
        let result = wallet.pay_invoice(&quote, invoice).await?;
        assert!(!result.0.paid);
        let mut tx = localstore.begin_tx().await?;
        assert_eq!(64, localstore.get_proofs(&mut tx).await?.total_amount());
        assert!(!result.0.paid);
        Ok(())
    }
}
