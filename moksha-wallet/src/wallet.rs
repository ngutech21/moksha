use moksha_core::{
    amount::Amount,
    blind::{BlindedMessage, BlindedSignature, BlindingFactor, TotalAmount},
    dhke::Dhke,
    keyset::KeysetId,
    primitives::{
        CurrencyUnit, MintInfoResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltBtcOnchainResponse, PostMeltQuoteBolt11Response, PostMeltQuoteBtcOnchainResponse,
        PostMintQuoteBolt11Response, PostMintQuoteBtcOnchainResponse,
    },
    proof::{Proof, Proofs},
    token::TokenV3,
};

use secp256k1::PublicKey;
use url::Url;

use crate::{
    client::CashuClient,
    error::MokshaWalletError,
    http::CrossPlatformHttpClient,
    localstore::{LocalStore, WalletKeyset},
    secret::DeterministicSecret,
};
use lightning_invoice::Bolt11Invoice as LNInvoice;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    vec,
};

#[derive(Clone)]
pub struct Wallet<L, C>
where
    L: LocalStore,
    C: CashuClient,
{
    client: C,
    dhke: Dhke,
    localstore: L,
    secret: DeterministicSecret,
}

pub struct WalletBuilder<L, C: CashuClient = CrossPlatformHttpClient>
where
    L: LocalStore,
    C: CashuClient + Default,
{
    client: Option<C>,
    localstore: Option<L>,
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

    pub async fn build(self) -> Result<Wallet<L, C>, MokshaWalletError> {
        let client = self.client.unwrap_or_default();
        let localstore = self.localstore.expect("localstore is required");

        let mut tx = localstore.begin_tx().await?;
        let seed_words = localstore.get_seed(&mut tx).await?;
        let seed = match seed_words {
            Some(seed) => seed,
            None => {
                let seed = DeterministicSecret::generate_random_seed_words()?;
                localstore.add_seed(&mut tx, &seed).await?;
                seed
            }
        };

        tx.commit().await?;

        Ok(Wallet::new(
            client as C,
            localstore,
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
    fn new(client: C, localstore: L, secret: DeterministicSecret) -> Self {
        Self {
            client,
            dhke: Dhke::new(),
            localstore,
            secret,
        }
    }

    pub fn builder() -> WalletBuilder<L, C> {
        WalletBuilder::default()
    }

    pub async fn create_quote_bolt11(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_mint_quote_bolt11(mint_url, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn create_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PostMintQuoteBtcOnchainResponse, MokshaWalletError> {
        self.client
            .post_mint_quote_onchain(mint_url, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn is_quote_paid(
        &self,
        mint_url: &Url,
        payment_method: &PaymentMethod,
        quote: String,
    ) -> Result<bool, MokshaWalletError> {
        Ok(match payment_method {
            PaymentMethod::Bolt11 => {
                self.client
                    .get_mint_quote_bolt11(mint_url, quote)
                    .await?
                    .paid
            }

            PaymentMethod::BtcOnchain => {
                self.client
                    .get_mint_quote_onchain(mint_url, quote)
                    .await?
                    .paid
            }
        })
    }

    pub async fn is_onchain_paid(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<bool, MokshaWalletError> {
        Ok(self
            .client
            .get_melt_quote_onchain(mint_url, quote)
            .await?
            .paid)
    }

    pub async fn is_onchain_tx_paid(
        &self,
        mint_url: &Url,
        txid: String,
    ) -> Result<bool, MokshaWalletError> {
        Ok(self.client.get_melt_onchain(mint_url, txid).await?.paid)
    }

    pub async fn get_wallet_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let keysets = self.localstore.get_keysets(&mut tx).await?;
        tx.commit().await?;
        Ok(keysets)
    }

    pub async fn get_mint_urls(&self) -> Result<Vec<Url>, MokshaWalletError> {
        let keysets = self.get_wallet_keysets().await?;
        let mints: HashSet<Url> = keysets.into_iter().map(|k| k.mint_url).collect();
        Ok(mints.into_iter().collect())
    }

    /// Stores the mints keys in the localstore
    pub async fn add_mint_keysets(
        &self,
        mint_url: &Url,
    ) -> Result<Vec<WalletKeyset>, MokshaWalletError> {
        if !self.client.is_v1_supported(mint_url).await? {
            return Err(MokshaWalletError::UnsupportedApiVersion);
        }

        // FIXME cleanup code
        let mint_keysets = self.client.get_keysets(mint_url).await?;

        let mut tx = self.localstore.begin_tx().await?;
        let mut result = vec![];
        for keyset in mint_keysets.keysets.iter() {
            let public_keys = self
                .client
                .get_keys_by_id(mint_url, keyset.id.clone())
                .await?
                .keysets
                .into_iter()
                .find(|k| k.id == keyset.id)
                .expect("no valid keyset found")
                .keys
                .clone();

            // ignore legacy keysets
            let keyset_id = match KeysetId::new(&keyset.id) {
                Ok(id) => id,
                Err(_) => {
                    continue;
                }
            };

            let wallet_keyset = WalletKeyset::new(
                &keyset_id,
                mint_url,
                &keyset.unit,
                0,
                public_keys,
                keyset.active,
            );
            result.push(wallet_keyset.clone());
            self.localstore
                .upsert_keyset(&mut tx, &wallet_keyset)
                .await?;
        }
        tx.commit().await?;
        Ok(result)
    }

    pub async fn get_balance(&self) -> Result<u64, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let total_amount = self.localstore.get_proofs(&mut tx).await?.total_amount();
        tx.commit().await?;
        Ok(total_amount)
    }

    pub async fn send_tokens(
        &self,
        wallet_keyset: &WalletKeyset,
        amount: u64,
    ) -> Result<TokenV3, MokshaWalletError> {
        let balance = self.get_balance().await?;
        if amount > balance {
            return Err(MokshaWalletError::NotEnoughTokens);
        }

        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self
            .localstore
            .get_proofs(&mut tx)
            .await?
            .proofs_by_keyset(&wallet_keyset.keyset_id);
        tx.commit().await?;

        let selected_proofs = all_proofs.proofs_for_amount(amount)?;
        let selected_tokens = (wallet_keyset.mint_url.to_owned(), selected_proofs.clone()).into();

        let (remaining_tokens, result) = self
            .split_tokens(wallet_keyset, &selected_tokens, amount.into())
            .await?;

        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .delete_proofs(&mut tx, &selected_proofs)
            .await?;

        self.localstore
            .add_proofs(&mut tx, &remaining_tokens.proofs())
            .await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn receive_tokens(
        &self,
        wallet_keyset: &WalletKeyset,
        tokens: &TokenV3,
    ) -> Result<(), MokshaWalletError> {
        let total_amount = tokens.total_amount();
        let (_, redeemed_tokens) = self
            .split_tokens(wallet_keyset, tokens, total_amount.into())
            .await?;
        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .add_proofs(&mut tx, &redeemed_tokens.proofs())
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_mint_quote(
        &self,
        mint_url: &Url,
        amount: Amount,
        currency: CurrencyUnit,
    ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_mint_quote_bolt11(mint_url, amount.0, currency)
            .await
    }

    pub async fn get_melt_quote_bolt11(
        &self,
        mint_url: &Url,
        invoice: String,
        currency: CurrencyUnit,
    ) -> Result<PostMeltQuoteBolt11Response, MokshaWalletError> {
        self.client
            .post_melt_quote_bolt11(mint_url, invoice.clone(), currency)
            .await
    }

    pub async fn pay_invoice(
        &self,
        wallet_keyset: &WalletKeyset,
        melt_quote: &PostMeltQuoteBolt11Response,
        invoice: String,
    ) -> Result<(PostMeltBolt11Response, u64), MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self.localstore.get_proofs(&mut tx).await?;
        tx.commit().await?;

        let ln_amount = Self::get_invoice_amount(&invoice)? + melt_quote.fee_reserve;

        if ln_amount > all_proofs.total_amount() {
            return Err(MokshaWalletError::NotEnoughTokens);
        }
        let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

        let total_proofs = {
            let selected_tokens =
                (wallet_keyset.mint_url.to_owned(), selected_proofs.clone()).into();
            let split_result = self
                .split_tokens(wallet_keyset, &selected_tokens, ln_amount.into())
                .await?;

            let mut tx = self.localstore.begin_tx().await?;
            self.localstore
                .delete_proofs(&mut tx, &selected_proofs)
                .await?;
            self.localstore
                .add_proofs(&mut tx, &split_result.0.proofs())
                .await?;
            tx.commit().await?;

            split_result.1.proofs()
        };

        let fee_blind = self
            .create_blank(melt_quote.fee_reserve.into(), &wallet_keyset.keyset_id)
            .await?;

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
            .map(|(msg, blinding_factor, _)| (msg.clone(), blinding_factor.clone()))
            .collect::<Vec<(BlindedMessage, BlindingFactor)>>();

        let mut tx = self.localstore.begin_tx().await?;
        match self
            .melt_token(
                &wallet_keyset.mint_url,
                melt_quote.to_owned().quote,
                &total_proofs,
                msgs,
            )
            .await
        {
            Ok(response) => {
                if !response.paid {
                    self.localstore.add_proofs(&mut tx, &total_proofs).await?;
                }
                let change_proofs = self.create_proofs_from_blinded_signatures(
                    &wallet_keyset.keyset_id,
                    &wallet_keyset.public_keys,
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
        mint_url: &Url,
        address: String,
        amount: u64,
    ) -> Result<Vec<PostMeltQuoteBtcOnchainResponse>, MokshaWalletError> {
        self.client
            .post_melt_quote_onchain(mint_url, address, amount, CurrencyUnit::Sat)
            .await
    }

    pub async fn pay_onchain(
        &self,
        wallet_keyset: &WalletKeyset,
        melt_quote: &PostMeltQuoteBtcOnchainResponse,
    ) -> Result<PostMeltBtcOnchainResponse, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_proofs = self.localstore.get_proofs(&mut tx).await?;
        tx.commit().await?;

        let ln_amount = melt_quote.amount + melt_quote.fee;

        if ln_amount > all_proofs.total_amount() {
            return Err(MokshaWalletError::NotEnoughTokens);
        }
        let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

        let mut tx = self.localstore.begin_tx().await?;
        let total_proofs = {
            let selected_tokens =
                (wallet_keyset.mint_url.to_owned(), selected_proofs.clone()).into();
            let split_result = self
                .split_tokens(wallet_keyset, &selected_tokens, ln_amount.into())
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
                &wallet_keyset.mint_url,
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
        keyset_id: &KeysetId,
        amount: u32,
    ) -> Result<Vec<(String, BlindingFactor)>, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_keysets = self.localstore.get_keysets(&mut tx).await?;
        let keyset = all_keysets
            .iter()
            .find(|k| k.keyset_id == *keyset_id)
            .expect("keyset not found");

        let start_index = (keyset.last_index + 1) as u32;
        let secret_range = self.secret.derive_range(keyset_id, start_index, amount)?;

        self.localstore
            .update_keyset_last_index(
                &mut tx,
                &WalletKeyset {
                    last_index: (start_index + amount - 1) as u64,
                    ..keyset.clone()
                },
            )
            .await?;
        tx.commit().await?;
        Ok(secret_range)
    }

    pub async fn split_tokens(
        &self,
        wallet_keyset: &WalletKeyset,
        tokens: &TokenV3,
        splt_amount: Amount,
    ) -> Result<(TokenV3, TokenV3), MokshaWalletError> {
        let total_token_amount = tokens.total_amount();
        let first_amount: Amount = (total_token_amount - splt_amount.0).into();
        let first_secrets = self
            .create_secrets(&wallet_keyset.keyset_id, first_amount.split().len() as u32)
            .await?;
        let first_outputs =
            self.create_blinded_messages(&wallet_keyset.keyset_id, first_amount, first_secrets.clone())?;

        // ############################################################################

        let second_amount = splt_amount.clone();
        let second_secrets = self
            .create_secrets(&wallet_keyset.keyset_id, second_amount.split().len() as u32)
            .await?;
        let second_outputs =
            self.create_blinded_messages(&wallet_keyset.keyset_id, second_amount, second_secrets.clone())?;

        let mut total_outputs = vec![];
        total_outputs.extend(get_blinded_msg(first_outputs.clone()));
        total_outputs.extend(get_blinded_msg(second_outputs.clone()));

        if tokens.total_amount() != total_outputs.total_amount() {
            return Err(MokshaWalletError::InvalidProofs);
        }

        let split_result = self
            .client
            .post_swap(&wallet_keyset.mint_url, tokens.proofs(), total_outputs)
            .await?;

        if split_result.signatures.is_empty() {
            return Ok((TokenV3::empty(), TokenV3::empty()));
        }

        let len_first = first_secrets.len();
        let secrets = [first_secrets, second_secrets].concat();
        let outputs = [first_outputs, second_outputs].concat();

        let secrets = secrets.into_iter().map(|(s, _)| s).collect::<Vec<String>>();

        let proofs = self
            .create_proofs_from_blinded_signatures(
                &wallet_keyset.keyset_id,
                &wallet_keyset.public_keys,
                split_result.signatures,
                secrets,
                outputs,
            )?
            .proofs();

        let first_tokens: TokenV3 = (
            wallet_keyset.mint_url.to_owned(),
            wallet_keyset.currency_unit.clone(),
            proofs[0..len_first].to_vec().into(),
        )
            .into();
        let second_tokens: TokenV3 = (
            wallet_keyset.mint_url.to_owned(),
            wallet_keyset.currency_unit.clone(),
            proofs[len_first..proofs.len()].to_vec().into(),
        )
            .into();

        if tokens.total_amount() != first_tokens.total_amount() + second_tokens.total_amount() {
            println!(
                "Error in swap: input {:?} != output {:?} + {:?}",
                tokens.total_amount(),
                first_tokens.total_amount(),
                second_tokens.total_amount()
            );
        }

        Ok((first_tokens, second_tokens))
    }

    pub async fn get_mint_info(
        &self,
        mint_url: &Url,
    ) -> Result<MintInfoResponse, MokshaWalletError> {
        self.client.get_info(mint_url).await
    }

    async fn melt_token(
        &self,
        mint_url: &Url,
        quote_id: String,
        proofs: &Proofs,
        fee_blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMeltBolt11Response, MokshaWalletError> {
        let melt_response = self
            .client
            .post_melt_bolt11(mint_url, proofs.clone(), quote_id, fee_blinded_messages)
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
        wallet_keyset: &WalletKeyset,
        payment_method: &PaymentMethod,
        amount: Amount,
        quote_id: String,
    ) -> Result<TokenV3, MokshaWalletError> {
        let split_amount = amount.split();

        let secret_range = self
            .create_secrets(&wallet_keyset.keyset_id, split_amount.len() as u32)
            .await?;
 
        let blinded_messages = split_amount
            .into_iter()
            .zip(secret_range)
            .map(|(amount, (secret, blinding_factor))| {
                let b_  = self
                    .dhke
                    .step1_alice(&secret, &blinding_factor)?;
                Ok((BlindedMessage {
                        amount,
                        b_,
                        id: wallet_keyset.keyset_id.to_string(), // FIXME use keyset_id
                    },
                    blinding_factor,
                    secret,))
            })
            .collect::<Result<Vec<(_, _, _)>,MokshaWalletError>>()?;

        let signatures = match payment_method {
            PaymentMethod::Bolt11 => {
                let post_mint_resp = self
                    .client
                    .post_mint_bolt11(
                        &wallet_keyset.mint_url,
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
                        &wallet_keyset.mint_url,
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
        let current_keyset_id = wallet_keyset.keyset_id.to_string(); // FIXME

        let proofs = signatures
            .iter()
            .zip(blinded_messages)
            .map(|(p, (_, priv_key, secret))| {
                let key = wallet_keyset
                    .public_keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys"); 
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset_id.clone())
            })
            .collect::<Vec<Proof>>()
            .into();

        let tokens: TokenV3 = (wallet_keyset.mint_url.to_owned(), proofs).into();
        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .add_proofs(&mut tx, &tokens.proofs())
            .await?;
        tx.commit().await?;

        Ok(tokens)
    }

    pub async fn create_blank(
        &self,
        fee_reserve: Amount,
        keyset_id: &KeysetId,
    ) -> Result<Vec<(BlindedMessage, BlindingFactor, String)>, MokshaWalletError> {
        if fee_reserve.0 == 0 {
            return Ok(vec![]);
        }

        let fee_reserve_float = fee_reserve.0 as f64;
        let count = (fee_reserve_float.log2().ceil() as u64).max(1);

        let secret_range = self.create_secrets(keyset_id, count as u32).await?;
        let blinded_messages = secret_range
            .into_iter()
            .map(|(secret, blinding_factor)| {
                let b_ = self
                    .dhke
                    .step1_alice(secret.clone(), &blinding_factor)?; 
                Ok((BlindedMessage {
                        amount: 1,
                        b_,
                        id: keyset_id.to_string(),
                    },
                    blinding_factor,
                    secret,))
            })
            .collect::<Result<Vec<(_, _, _)>,MokshaWalletError>>()?;

        Ok(blinded_messages)
    }


    fn create_blinded_messages(
        &self,
        keyset_id: &KeysetId,
        amount: Amount,
        secrets_factors: Vec<(String, BlindingFactor)>,
    ) -> Result<Vec<(BlindedMessage, BlindingFactor)>, MokshaWalletError> {
        let split_amount = amount.split();

        split_amount
            .into_iter()
            .zip(secrets_factors)
            .map(|(amount, (secret, blinding_factor))| {
                let b_  = self
                    .dhke
                    .step1_alice(secret, &blinding_factor)?;
                Ok((BlindedMessage {
                        amount,
                        b_,
                        id: keyset_id.to_string(),
                    },
                    blinding_factor,))
            })
            .collect::<Result<Vec<(_, _)>,MokshaWalletError>>()
    }

    fn create_proofs_from_blinded_signatures(
        &self,
        keyset_id: &KeysetId,
        pub_keys: &HashMap<u64, PublicKey>,
        signatures: Vec<BlindedSignature>,
        secrets: Vec<String>,
        outputs: Vec<(BlindedMessage, BlindingFactor)>,
    ) -> Result<Proofs, MokshaWalletError> {
        let current_keyset_id = keyset_id.to_string(); // FIXME

        let blinding_factors = outputs
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<BlindingFactor>>();

        Ok(signatures
            .iter()
            .zip(blinding_factors)
            .zip(secrets)
            .map(|((p, blinding_factor), secret)| {
                let key = pub_keys
                    .get(&p.amount)
                    .ok_or(MokshaWalletError::PubkeyNotFound)?;
                let pub_alice = self.dhke.step3_alice(p.c_, blinding_factor.to_owned(), *key)?;
                Ok(Proof::new(p.amount, secret, pub_alice, current_keyset_id.clone()))
            })
            .collect::<Result<Vec<_>,MokshaWalletError>>()?
            .into())
    }

    pub async fn get_proofs(&self) -> Result<Proofs, MokshaWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let proofs = self.localstore.get_proofs(&mut tx).await?;
        tx.commit().await?;
        Ok(proofs)
    }
}

// FIXME implement for Vec<BlindedMessage, Secretkey>
fn get_blinded_msg(blinded_messages: Vec<(BlindedMessage, BlindingFactor)>) -> Vec<BlindedMessage> {
    blinded_messages
        .into_iter()
        .map(|(msg, _)| msg)
        .collect::<Vec<BlindedMessage>>()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::client::MockCashuClient;
    use crate::localstore::sqlite::SqliteLocalStore;
    use crate::localstore::{LocalStore, WalletKeyset};
    use crate::wallet::WalletBuilder;

    use moksha_core::fixture::{read_fixture, read_fixture_as};
    use moksha_core::keyset::{KeysetId, MintKeyset, Keysets};
    use moksha_core::primitives::{
        CurrencyUnit, KeyResponse, KeysResponse, PaymentMethod, PostMeltBolt11Response,
        PostMeltQuoteBolt11Response, PostMintBolt11Response, PostSwapResponse,
    };

    use moksha_core::token::TokenV3;
    use secp256k1::PublicKey;
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
        let keysets = Keysets::new(keys.keyset_id, CurrencyUnit::Sat, true);

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
    async fn test_blank_blinded_messages_1000_sats() -> anyhow::Result<()> {
        let localstore = SqliteLocalStore::with_in_memory().await?;
        let wallet_keyset = create_test_wallet_keyset()?;
        let mut tx = localstore.begin_tx().await?;
        localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
        tx.commit().await?;

        let client = create_mock();

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?;
        let result = wallet
            .create_blank(1000.into(), &KeysetId::new("00d31cecf59d18c0")?)
            .await;
        println!("{:?}", result);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.len() == 10);
        assert!(result.first().unwrap().0.amount == 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_blank_blinded_messages_zero_sats() -> anyhow::Result<()> {
        let localstore = SqliteLocalStore::with_in_memory().await?;
        let wallet_keyset = create_test_wallet_keyset()?;
        let mut tx = localstore.begin_tx().await?;
        localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
        tx.commit().await?;

        let client = create_mock();

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?;
        let result = wallet
            .create_blank(0.into(), &KeysetId::new("00d31cecf59d18c0")?)
            .await;
        println!("{:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_blank_blinded_messages_serialize() -> anyhow::Result<()> {
        let localstore = SqliteLocalStore::with_in_memory().await?;
        let wallet_keyset = create_test_wallet_keyset()?;
        let mut tx = localstore.begin_tx().await?;
        localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
        tx.commit().await?;

        let client = create_mock();

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?;

        let result = wallet
            .create_blank(4000.into(), &KeysetId::new("00d31cecf59d18c0")?)
            .await?;
        for (blinded_message, _, _) in result {
            let out = serde_json::to_string(&blinded_message)?;
            assert!(!out.is_empty());
        }
        Ok(())
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
        let wallet_keyset = create_test_wallet_keyset()?;
        let mut tx = localstore.begin_tx().await?;
        localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
        tx.commit().await?;

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?;

        let result = wallet
            .mint_tokens(
                &wallet_keyset,
                &PaymentMethod::Bolt11,
                20.into(),
                "hash".to_string(),
            )
            .await?;
        assert_eq!(20, result.total_amount());
        result.tokens.into_iter().for_each(|t| {
            assert_eq!(wallet_keyset.mint_url, t.mint.expect("mint is empty"));
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
        let keyset = create_test_wallet_keyset()?;
        let mut tx = localstore.begin_tx().await?;
        localstore.upsert_keyset(&mut tx, &keyset).await?;
        tx.commit().await?;

        let wallet = WalletBuilder::new()
            .with_client(client)
            .with_localstore(localstore)
            .build()
            .await?;

        let tokens = read_fixture("token_64.cashu")?.try_into()?;
        let result = wallet.split_tokens(&keyset, &tokens, 20.into()).await?;

        let first = result.0;

        assert_eq!(CurrencyUnit::Sat, first.clone().unit.unwrap());
        assert_eq!(24, first.total_amount());

        let second = result.1;

        assert_eq!(CurrencyUnit::Sat, second.clone().unit.unwrap());
        assert_eq!(40, second.total_amount());
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

        let wallet = WalletBuilder::new()
            .with_client(create_mock())
            .with_localstore(local_store)
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
        let wallet_keyset = create_test_wallet_keyset()?;
        local_store.upsert_keyset(&mut tx, &wallet_keyset).await?;
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
            .build()
            .await?;

        // FIXME fix mock response: Warning Error in swap: input 32 != output 24 + 40

        // 21 sats
        let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

        let quote = wallet
            .get_melt_quote_bolt11(&mint_url, invoice.clone(), CurrencyUnit::Sat)
            .await?;
        let result = wallet.pay_invoice(&wallet_keyset, &quote, invoice).await?;
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
        let wallet_keyset = create_test_wallet_keyset()?;
        localstore.upsert_keyset(&mut tx, &wallet_keyset).await?;
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
            .build()
            .await?;

        // 21 sats
        let invoice = "lnbcrt210n1pjg6mqhpp5pza5wzh0csjjuvfpjpv4zdjmg30vedj9ycv5tyfes9x7dp8axy0sdqqcqzzsxqyz5vqsp5vtxg4c5tw2s2zxxya2a7an0psn9mcfmlqctxzntm3sngnpyk3muq9qyyssqf8z5f90yu3wrmsufnnza25qjlnvc6ukdr094ckzn63ktcy6z5fw5mxf9skndpg2p4648gfjfvvx4qg2lqvlryyycg5k7x9h4dw70t4qq37pegm".to_string();

        let quote = wallet
            .get_melt_quote_bolt11(&wallet_keyset.mint_url, invoice.clone(), CurrencyUnit::Sat)
            .await?;

        let wallet_keyset = create_test_wallet_keyset()?;

        let result = wallet.pay_invoice(&wallet_keyset, &quote, invoice).await?;
        assert!(!result.0.paid);
        let mut tx = localstore.begin_tx().await?;
        assert_eq!(64, localstore.get_proofs(&mut tx).await?.total_amount());
        assert!(!result.0.paid);
        Ok(())
    }

    fn create_test_wallet_keyset() -> anyhow::Result<WalletKeyset> {
        let pub_keys = read_fixture_as::<HashMap<u64, PublicKey>>("pub_keys.json")?;
        let keyset_id = KeysetId::new("00d31cecf59d18c0")?;
        let mint_url = Url::parse("http://127.0.0.1:3338")?;

        let wallet_keyset = WalletKeyset::new(
            &keyset_id,
            &mint_url,
            &CurrencyUnit::Sat,
            0,
            pub_keys.clone(),
            true,
        );
        Ok(wallet_keyset)
    }
}
