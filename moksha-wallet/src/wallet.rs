use moksha_core::{
    amount::Amount,
    blind::{BlindedMessage, BlindedSignature, TotalAmount},
    dhke::Dhke,
    keyset::V1Keyset,
    primitives::{
        CurrencyUnit, KeyResponse, MintInfoResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltOnchainResponse, PostMeltQuoteBolt11Response, PostMeltQuoteOnchainResponse,
        PostMintQuoteBolt11Response, PostMintQuoteOnchainResponse,
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

        let load_keysets = localstore.get_keysets().await?;

        let mint_keysets = client.get_keysets(&mint_url).await?;
        if load_keysets.is_empty() {
            let wallet_keysets = mint_keysets
                .keysets
                .iter()
                .map(|m| WalletKeyset {
                    id: m.clone().id,
                    mint_url: mint_url.to_string(),
                })
                .collect::<Vec<WalletKeyset>>();

            for wkeyset in wallet_keysets {
                localstore.add_keyset(&wkeyset).await?;
            }
        }

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
    ) -> Self {
        Self {
            client,
            keyset_id: mint_keys,
            keyset: key_response,
            dhke: Dhke::new(),
            localstore,
            mint_url,
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
    ) -> Result<PostMintQuoteOnchainResponse, MokshaWalletError> {
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
        Ok(self.localstore.get_proofs().await?.total_amount())
    }

    pub async fn send_tokens(&self, amount: u64) -> Result<TokenV3, MokshaWalletError> {
        let balance = self.get_balance().await?;
        if amount > balance {
            return Err(MokshaWalletError::NotEnoughTokens);
        }

        let all_proofs = self.localstore.get_proofs().await?;
        let selected_proofs = all_proofs.proofs_for_amount(amount)?;
        let selected_tokens = (self.mint_url.to_owned(), selected_proofs.clone()).into();

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
        let all_proofs = self.localstore.get_proofs().await?;

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

            // FIXME create transaction
            self.localstore.delete_proofs(&selected_proofs).await?;
            self.localstore.add_proofs(&split_result.0.proofs()).await?;

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

        match self
            .melt_token(melt_quote.to_owned().quote, ln_amount, &total_proofs, msgs)
            .await
        {
            Ok(response) => {
                if !response.paid {
                    self.localstore.add_proofs(&total_proofs).await?;
                }
                let change_proofs = self.create_proofs_from_blinded_signatures(
                    response.clone().change,
                    secrets,
                    outputs,
                )?;
                self.localstore.add_proofs(&change_proofs).await?;

                Ok((response, change_proofs.total_amount()))
            }
            Err(e) => {
                self.localstore.add_proofs(&total_proofs).await?;
                Err(e)
            }
        }
    }

    pub async fn get_melt_quote_btconchain(
        &self,
        address: String,
        amount: u64,
    ) -> Result<Vec<PostMeltQuoteOnchainResponse>, MokshaWalletError> {
        self.client
            .post_melt_quote_onchain(&self.mint_url, address, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn pay_onchain(
        &self,

        melt_quote: &PostMeltQuoteOnchainResponse,
    ) -> Result<PostMeltOnchainResponse, MokshaWalletError> {
        let all_proofs = self.localstore.get_proofs().await?;

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

            // FIXME create transaction
            self.localstore.delete_proofs(&selected_proofs).await?;
            self.localstore.add_proofs(&split_result.0.proofs()).await?;

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
            self.localstore.delete_proofs(&total_proofs).await?;
        }
        Ok(melt_response)

        // match self
        //     .melt_token(melt_quote.quote, ln_amount, &total_proofs, msgs)
        //     .await
        // {
        //     Ok(response) => {
        //         if !response.paid {
        //             self.localstore.add_proofs(&total_proofs).await?;
        //         }
        //         let change_proofs = self.create_proofs_from_blinded_signatures(
        //             response.clone().change,
        //             secrets,
        //             outputs,
        //         )?;
        //         self.localstore.add_proofs(&change_proofs).await?;

        //         Ok(response)
        //     }
        //     Err(e) => {
        //         self.localstore.add_proofs(&total_proofs).await?;
        //         Err(e)
        //     }
        // }
    }

    pub async fn split_tokens(
        &self,
        tokens: &TokenV3,
        splt_amount: Amount,
    ) -> Result<(TokenV3, TokenV3), MokshaWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount: Amount = (total_token_amount - splt_amount.0).into();
        let first_secrets = first_amount.split().create_secrets();
        let first_outputs = self.create_blinded_messages(first_amount, &first_secrets)?;

        // ############################################################################

        let second_amount = splt_amount.clone();
        let second_secrets = second_amount.split().create_secrets();
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
        payment_method: &PaymentMethod,
        amount: Amount,
        quote_id: String,
    ) -> Result<TokenV3, MokshaWalletError> {
        let split_amount = amount.split();
        let secrets = split_amount.create_secrets();

        let blinded_messages = split_amount
            .into_iter()
            .zip(secrets.clone())
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
                (
                    BlindedMessage {
                        amount,
                        b_,
                        id: self.keyset_id.clone().id,
                    },
                    alice_secret_key,
                )
            })
            .collect::<Vec<(BlindedMessage, SecretKey)>>();

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
                            .map(|(msg, _)| msg)
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
                            .map(|(msg, _)| msg)
                            .collect::<Vec<BlindedMessage>>(),
                    )
                    .await?;
                post_mint_resp.signatures
            }
        };

        // step 3: unblind signatures
        let current_keyset_id = self.keyset_id.clone().id; // FIXME

        let private_keys = blinded_messages
            .clone()
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<SecretKey>>();

        let proofs = signatures
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
            .into();

        let tokens: TokenV3 = (self.mint_url.to_owned(), proofs).into();
        self.localstore.add_proofs(&tokens.proofs()).await?;

        Ok(tokens)
    }

    // FIXME implement for Amount
    fn create_blinded_messages(
        &self,
        amount: Amount,
        secrets: &[String],
    ) -> Result<Vec<(BlindedMessage, SecretKey)>, MokshaWalletError> {
        let split_amount = amount.split();

        Ok(split_amount
            .into_iter()
            .zip(secrets)
            .map(|(amount, secret)| {
                let (b_, alice_secret_key) = self.dhke.step1_alice(secret, None).unwrap(); // FIXME
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
    use crate::wallet::WalletBuilder;
    use crate::{
        error::MokshaWalletError,
        localstore::{LocalStore, WalletKeyset},
    };
    use async_trait::async_trait;

    use moksha_core::fixture::{read_fixture, read_fixture_as};
    use moksha_core::keyset::{MintKeyset, V1Keysets};
    use moksha_core::primitives::{
        CurrencyUnit, KeyResponse, KeysResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltQuoteBolt11Response, PostMintBolt11Response, PostSwapResponse,
    };
    use moksha_core::proof::Proofs;
    use moksha_core::token::{Token, TokenV3};
    use url::Url;

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
                    mint: Some(Url::parse("http://127.0.0.1:3338").expect("invalid url")),
                    proofs: Proofs::empty(),
                }),
            }
        }
    }

    #[async_trait(?Send)]
    impl LocalStore for MockLocalStore {
        async fn add_proofs(&self, _: &Proofs) -> Result<(), crate::error::MokshaWalletError> {
            Ok(())
        }

        async fn get_proofs(
            &self,
        ) -> Result<moksha_core::proof::Proofs, crate::error::MokshaWalletError> {
            Ok(self.tokens.clone().proofs())
        }

        async fn delete_proofs(
            &self,
            _proofs: &Proofs,
        ) -> Result<(), crate::error::MokshaWalletError> {
            Ok(())
        }

        async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
            Ok(vec![])
        }

        async fn add_keyset(&self, _keyset: &WalletKeyset) -> Result<(), MokshaWalletError> {
            Ok(())
        }
    }

    fn create_mock() -> MockCashuClient {
        let keys = MintKeyset::new("mykey", "");
        let key_response = KeyResponse {
            keys: keys.public_keys.clone(),
            id: keys.keyset_id.clone(),
            unit: CurrencyUnit::Sat,
        };
        let keys_response = KeysResponse::new(key_response);
        let keysets = V1Keysets::new(keys.keyset_id, CurrencyUnit::Sat, true);

        let mut client = MockCashuClient::default();
        client
            .expect_get_keys()
            .returning(move |_| Ok(keys_response.clone()));
        client
            .expect_get_keysets()
            .returning(move |_| Ok(keysets.clone()));
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

        let localstore = MockLocalStore::default();
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
        let localstore = MockLocalStore::default();

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
        let local_store = MockLocalStore::with_tokens(fixture.try_into()?);

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

    #[tokio::test]
    async fn test_pay_invoice() -> anyhow::Result<()> {
        let fixture = read_fixture("token_60.cashu")?; // 60 tokens (4,8,16,32)
        let local_store = MockLocalStore::with_tokens(fixture.try_into()?);

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

        let tmp = tempfile::tempdir().expect("Could not create tmp dir for wallet");
        let tmp_dir = tmp
            .path()
            .to_str()
            .expect("Could not create tmp dir for wallet");

        let localstore = SqliteLocalStore::with_path(format!("{tmp_dir}/test_wallet.db"))
            .await
            .expect("Could not create localstore");

        localstore.add_proofs(&tokens.proofs()).await?;
        assert_eq!(64, localstore.get_proofs().await?.total_amount());

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
        assert_eq!(64, localstore.get_proofs().await?.total_amount());
        assert!(!result.0.paid);
        Ok(())
    }
}
