use clap::{Parser, Subcommand};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use moksha_core::primitives::{
    CurrencyUnit, PaymentMethod, PostMeltBtcOnchainResponse, PostMintQuoteBolt11Response,
    PostMintQuoteBtcOnchainResponse,
};
use moksha_core::token::TokenV3;
use moksha_wallet::client::CashuClient;

use moksha_wallet::http::CrossPlatformHttpClient;

use moksha_wallet::localstore::WalletKeysetFilter;
use mokshacli::cli::{self, choose_mint, get_mints_with_balance};
use num_format::{Locale, ToFormattedString};
use qrcode::render::unicode;
use qrcode::QrCode;

use std::path::PathBuf;
use std::str::FromStr;

use url::Url;

#[derive(Parser)]
#[command(arg_required_else_help(true))]
struct Opts {
    #[clap(short, long)]
    db_dir: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    /// Mint tokens
    Mint { amount: u64 },

    /// Pay Lightning invoice
    Pay { invoice: String },

    /// Pay Bitcoin on chain
    PayOnchain { address: String, amount: u64 },

    /// Send tokens
    Send { amount: u64 },

    /// Receive tokens
    Receive { token: String },

    /// Show local balance
    Balance,

    /// Show version and configuration
    Info,

    /// Add a new mint to the wallet
    AddMint { mint_url: Url },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use moksha_wallet::localstore::sqlite::SqliteLocalStore;

    let cli = Opts::parse();

    let db_path = match cli.db_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir.clone())?;
            dir.join("wallet.db").to_str().unwrap().to_string()
        }

        None => moksha_wallet::config_path::db_path(),
    };

    let term = Term::stdout();
    let localstore = SqliteLocalStore::with_path(db_path.clone()).await?;
    let client = CrossPlatformHttpClient::new();
    let wallet = moksha_wallet::wallet::WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await
        .map_err(|e| {
            if matches!(
                e,
                moksha_wallet::error::MokshaWalletError::UnsupportedApiVersion
            ) {
                term.write_line("Error: Mint does not support /v1 api")
                    .expect("write_line failed");
                std::process::exit(1);
            }
            e
        })?;

    match cli.command {
        Command::AddMint { mint_url } => {
            wallet.add_mint_keysets(&mint_url).await?;
            term.write_line("Mint added successfully ")?;
        }
        Command::Info => {
            let wallet_version = style(env!("CARGO_PKG_VERSION")).cyan();
            let mint_urls = wallet.get_mint_urls().await?;
            let db_path = style(db_path).cyan();
            term.write_line(&format!("Version: {wallet_version}"))?;
            term.write_line(&format!("DB: {db_path}"))?;

            if mint_urls.is_empty() {
                term.write_line("No mints found.")?;
            } else {
                term.write_line("Mints:")?;
                for mint in mint_urls {
                    term.write_line(&format!(" - {}", mint))?;
                }
            }
        }
        // checks if the mints keyset is already in the wallet, if not it adds it and then imports the tokens
        Command::Receive { token } => {
            let token: TokenV3 = TokenV3::from_str(&token)?;
            let mint_urls = wallet.get_mint_urls().await?;
            let currency = match &token.currency_unit {
                Some(currency) => currency,
                None => &CurrencyUnit::Sat,
            };

            let token_mint_url = match token.mint() {
                Some(url) => url,
                None => {
                    term.write_line("Invalid Token: Missing mint url")?;
                    return Ok(());
                }
            };

            if !mint_urls.contains(&token_mint_url) {
                let add_mint = Confirm::new()
                    .with_prompt(format!(
                        "New mint found {:?} . Do you want to add it?",
                        token_mint_url.to_string()
                    ))
                    .interact()?;

                if !add_mint {
                    return Ok(());
                }

                let is_valid_mint = CrossPlatformHttpClient::new()
                    .is_v1_supported(&token_mint_url)
                    .await?;
                if !is_valid_mint {
                    term.write_line("Error: Invalid mint url")?;
                    return Ok(());
                }

                wallet.add_mint_keysets(&token_mint_url).await?;
            }

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .get_active(&token_mint_url, currency)
                .expect("no active keyset found");

            wallet.receive_tokens(wallet_keyset, &token).await?;
            cli::show_total_balance(&wallet).await?;
        }
        Command::Send { amount } => {
            let currency_unit = CurrencyUnit::Sat;
            let mint_url = choose_mint(&wallet, &currency_unit).await?;

            if mint_url.1 < amount {
                term.write_line("Error: Not enough tokens in selected mint")?;
                return Ok(());
            }

            let mint_url = mint_url.0;

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .get_active(&mint_url, &currency_unit)
                .expect("no active keyset found");

            term.write_line(&format!("Using tokens from mint: {mint_url}"))?;
            let result = wallet.send_tokens(wallet_keyset, amount).await?;
            let tokens: String = result.try_into()?;

            term.write_line(&format!("Result {amount} (sat):\n{tokens}"))?;
            cli::show_total_balance(&wallet).await?;
        }
        Command::Balance => {
            let total_balance = wallet.get_balance().await?;
            if total_balance > 0 {
                let mints = get_mints_with_balance(&wallet, &CurrencyUnit::Sat).await?;
                term.write_line(&format!(
                    "You have balances in {} mints",
                    style(mints.len()).cyan()
                ))?;

                for mint in mints {
                    term.write_line(&format!(
                        " - {} {} (sat)",
                        mint.0,
                        style(mint.1.to_formatted_string(&Locale::en)).cyan()
                    ))?;
                }
            }
            cli::show_total_balance(&wallet).await?;
        }
        Command::Pay { invoice } => {
            let currency_unit = CurrencyUnit::Sat;
            let mint_url = choose_mint(&wallet, &currency_unit).await?.0;
            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .get_active(&mint_url, &currency_unit)
                .expect("no active keyset found");

            let quote = wallet
                .get_melt_quote_bolt11(&mint_url, invoice.clone(), currency_unit)
                .await?;

            let pay_confirmed = Confirm::new()
                .with_prompt(format!(
                    "Pay lightning invoice: amount {} + fee {} = {} (sat)?",
                    quote.amount,
                    quote.fee_reserve,
                    quote.amount + quote.fee_reserve
                ))
                .interact()?;

            if !pay_confirmed {
                return Ok(());
            }

            let response = wallet.pay_invoice(wallet_keyset, &quote, invoice).await?;

            // FIXME handle not enough tokens error

            if response.0.paid {
                if response.1 > 0 {
                    term.write_line(&format!(
                        "Returned fees {} (sat)",
                        response.1.to_formatted_string(&Locale::en)
                    ))?;
                }
                term.write_line("\nInvoice has been paid: Tokens melted successfully")?;
                cli::show_total_balance(&wallet).await?;
            } else {
                term.write_line("Error: Tokens not melted")?;
            }
        }
        Command::PayOnchain { address, amount } => {
            // FIXME remove redundant code
            let currency = CurrencyUnit::Sat;
            let mint_url = choose_mint(&wallet, &currency).await?;

            if mint_url.1 < amount {
                term.write_line("Error: Not enough tokens in selected mint")?;
                return Ok(());
            }
            let mint_url = mint_url.0;

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .get_active(&mint_url, &currency)
                .expect("Keyset not found");

            let info = wallet.get_mint_info(&mint_url).await?;

            if info.nuts.nut19.map_or(true, |nut18| !nut18.supported) {
                term.write_line("Error: onchain-payments are not supported by this mint")?;
                return Ok(());
            }

            let quotes = wallet
                .get_melt_quote_btconchain(&mint_url, address.clone(), amount)
                .await?;

            if quotes.is_empty() {
                term.write_line("Error: No quotes found")?;
                return Ok(());
            }

            let quote = quotes.first().expect("No quotes found");

            term.write_line(&format!(
                "Create onchain transaction to melt tokens: amount {} + fee {} = {} (sat)\n{}\n",
                amount,
                quote.fee,
                amount + quote.fee,
                address
            ))?;

            let pay_confirmed = Confirm::new().with_prompt("Confirm payment?").interact()?;

            if !pay_confirmed {
                return Ok(());
            }

            let PostMeltBtcOnchainResponse { paid, txid } =
                wallet.pay_onchain(wallet_keyset, quote).await?;
            if let Some(txid) = txid.clone() {
                term.write_line(&format!("Created transaction: {}\n", &txid))?;
            }

            let progress_bar = cli::progress_bar()?;
            progress_bar.set_message("Waiting for payment confirmation ...");

            loop {
                tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;

                if paid
                    || wallet
                        .is_onchain_paid(&mint_url, quote.quote.clone())
                        .await?
                {
                    progress_bar.finish_with_message("\nTokens melted successfully\n");
                    cli::show_total_balance(&wallet).await?;
                    break;
                } else {
                    continue;
                }
            }
        }
        Command::Mint { amount } => {
            let currency = CurrencyUnit::Sat;
            let mint_url = choose_mint(&wallet, &currency).await?.0;

            let info = wallet.get_mint_info(&mint_url).await?;

            let payment_method = info.nuts.nut18.as_ref().map_or_else(
                || {
                    term.write_line("Only bolt11 minting is supported")
                        .expect("write_line failed");
                    PaymentMethod::Bolt11
                },
                |nut17| {
                    if !nut17.supported {
                        term.write_line("Only bolt11 minting is supported")
                            .expect("write_line failed");
                        PaymentMethod::Bolt11
                    } else {
                        let selections = &[PaymentMethod::BtcOnchain, PaymentMethod::Bolt11];

                        let selection = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose a payment method:")
                            .default(0)
                            .items(&selections[..])
                            .interact()
                            .expect("Selection failed");
                        selections[selection].clone()
                    }
                },
            );

            let quote = match payment_method {
                PaymentMethod::BtcOnchain => {
                    let nut17 = info.nuts.nut18.expect("nut17 is None");
                    let payment_method = nut17.payment_methods.first().expect("no payment methods");

                    if amount < payment_method.min_amount {
                        term.write_line(&format!(
                            "Amount too low. Minimum amount is {} (sat)",
                            payment_method.min_amount.to_formatted_string(&Locale::en)
                        ))?;
                        return Ok(());
                    }

                    if amount > payment_method.max_amount {
                        term.write_line(&format!(
                            "Amount too high. Maximum amount is {} (sat)",
                            payment_method.max_amount.to_formatted_string(&Locale::en)
                        ))?;
                        return Ok(());
                    }

                    let PostMintQuoteBtcOnchainResponse { address, quote, .. } =
                        wallet.create_quote_onchain(&mint_url, amount).await?;

                    term.write_line(&format!("Pay onchain to mint tokens:\n\n{address}"))?;

                    let amount_btc = amount as f64 / 100_000_000.0;
                    let bip21_code = format!("bitcoin:{}?amount={}", address, amount_btc);
                    let image = QrCode::new(bip21_code)?
                        .render::<unicode::Dense1x2>()
                        .quiet_zone(true)
                        .build();
                    term.write_line(&image)?;
                    quote
                }
                PaymentMethod::Bolt11 => {
                    let PostMintQuoteBolt11Response {
                        payment_request,
                        quote,
                        ..
                    } = wallet.create_quote_bolt11(&mint_url, amount).await?;

                    term.write_line(&format!(
                        "Pay lightning invoice to mint tokens:\n\n{payment_request}"
                    ))?;

                    let image = QrCode::new(payment_request)?
                        .render::<unicode::Dense1x2>()
                        .quiet_zone(true)
                        .build();
                    term.write_line(&image)?;

                    quote
                }
            };

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .get_active(&mint_url, &currency)
                .expect("Keyset not found");

            let progress_bar = cli::progress_bar()?;
            progress_bar.set_message("Waiting for payment ...");

            loop {
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(500),
                )
                .await;

                if !wallet
                    .is_quote_paid(&mint_url, &payment_method, quote.clone())
                    .await?
                {
                    continue;
                }

                // FIXME store quote in db and add option to retry minting later

                let mint_result = wallet
                    .mint_tokens(wallet_keyset, &payment_method, amount.into(), quote.clone())
                    .await;

                match mint_result {
                    Ok(_) => {
                        progress_bar.finish_with_message("Tokens minted successfully.\n");
                        cli::show_total_balance(&wallet).await?;
                        break;
                    }
                    Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(_, _)) => {
                        continue;
                    }
                    Err(e) => {
                        term.write_line(&format!("General Error: {}", e))?;
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
