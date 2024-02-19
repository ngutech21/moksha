use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use moksha_core::primitives::{
    CurrencyUnit, PaymentMethod, PostMeltOnchainResponse, PostMintQuoteBolt11Response,
    PostMintQuoteOnchainResponse,
};
use moksha_wallet::http::CrossPlatformHttpClient;
use num_format::{Locale, ToFormattedString};
use std::io::Write;
use std::{io::stdout, path::PathBuf};
use url::Url;

#[derive(Parser)]
#[command(version)]
struct Opts {
    #[clap(short, long)]
    mint_url: Url,

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

    let localstore = SqliteLocalStore::with_path(db_path.clone()).await?;
    let client = CrossPlatformHttpClient::new();
    let wallet = moksha_wallet::wallet::WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .with_mint_url(cli.mint_url.clone())
        .build()
        .await
        .map_err(|e| {
            if matches!(
                e,
                moksha_wallet::error::MokshaWalletError::UnsupportedApiVersion
            ) {
                println!("Error: Mint does not support /v1 api");
                std::process::exit(1);
            }
            e
        })?;

    match cli.command {
        Command::Info => {
            let wallet_version = env!("CARGO_PKG_VERSION");
            println!(
                "Version: {}\nDB: {}\nMint URL: {}",
                wallet_version, db_path, cli.mint_url,
            );
        }
        Command::Receive { token } => {
            wallet.receive_tokens(&token.try_into()?).await?;
            println!(
                "Tokens received successfully.\nNew balance {} (sat)",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }
        Command::Send { amount } => {
            let result = wallet.send_tokens(amount).await?;
            let ser: String = result.try_into()?;

            println!("Result {amount} (sat):\n{ser}");
            println!(
                "\nNew balance: {} (sat)",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }

        Command::Balance => {
            let balance = wallet.get_balance().await?.to_formatted_string(&Locale::en);
            println!("Balance: {balance} (sat)");
        }
        Command::Pay { invoice } => {
            let quote = wallet
                .get_melt_quote_bolt11(invoice.clone(), CurrencyUnit::Sat)
                .await?;

            let pay_confirmed = Confirm::new()
                .with_prompt(format!(
                    "Pay lightning invoice: amount {} + fee {} = {} (sat)?",
                    quote.amount,
                    quote.fee_reserve,
                    quote.amount + quote.fee_reserve
                ))
                .interact()
                .unwrap();

            if !pay_confirmed {
                return Ok(());
            }

            let response = wallet.pay_invoice(&quote, invoice).await?;

            // FIXME handle not enough tokens error

            if response.0.paid {
                if response.1 > 0 {
                    println!(
                        "Returned fees {} (sat)",
                        response.1.to_formatted_string(&Locale::en)
                    );
                }
                println!(
                    "\nInvoice has been paid: Tokens melted successfully\nNew balance: {} (sat)",
                    wallet.get_balance().await?.to_formatted_string(&Locale::en)
                );
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::PayOnchain { address, amount } => {
            let info = wallet.get_mint_info().await?;

            if info.nuts.nut15.map_or(true, |nut15| !nut15.supported) {
                println!("Error: onchain-payments are not supported by this mint");
                return Ok(());
            }

            let quotes = wallet
                .get_melt_quote_btconchain(address.clone(), amount)
                .await?;

            if quotes.is_empty() {
                println!("Error: No quotes found");
                return Ok(());
            }

            let quote = quotes.first().expect("No quotes found");

            println!(
                "Create onchain transaction to melt tokens: amount {} + fee {} = {} (sat)\n\n{}",
                amount,
                quote.fee,
                amount + quote.fee,
                address
            );
            let pay_confirmed = Confirm::new()
                .with_prompt("Confirm payment?")
                .interact()
                .unwrap();

            if !pay_confirmed {
                return Ok(());
            }

            let PostMeltOnchainResponse { paid, txid } = wallet.pay_onchain(quote).await?;
            println!("Created transaction: {}\n", &txid);

            let mut lock = stdout().lock();
            loop {
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(2_000),
                )
                .await;

                if paid || wallet.is_onchain_tx_paid(txid.clone()).await? {
                    println!(
                        "\nTokens melted successfully\nNew balance: {} (sat)",
                        wallet.get_balance().await?.to_formatted_string(&Locale::en)
                    );
                    break;
                } else {
                    write!(lock, ".").unwrap();
                    lock.flush().unwrap();
                    continue;
                }
            }
        }
        Command::Mint { amount } => {
            let info = wallet.get_mint_info().await?;

            let payment_method = info.nuts.nut14.as_ref().map_or_else(
                || {
                    println!("Only bolt11 minting is supported");
                    PaymentMethod::Bolt11
                },
                |nut14| {
                    if !nut14.supported {
                        println!("Only bolt11 minting is supported");
                        PaymentMethod::Bolt11
                    } else {
                        let selections = &[PaymentMethod::BtcOnchain, PaymentMethod::Bolt11];

                        let selection = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose a payment method:")
                            .default(0)
                            .items(&selections[..])
                            .interact()
                            .unwrap();
                        selections[selection].clone()
                    }
                },
            );

            let quote = match payment_method {
                PaymentMethod::BtcOnchain => {
                    let nut14 = info.nuts.nut14.expect("nut14 is None");
                    let payment_method = nut14.payment_methods.first().expect("no payment methods");
                    if amount < payment_method.min_amount {
                        println!(
                            "Amount too low. Minimum amount is {} (sat)",
                            payment_method.min_amount.to_formatted_string(&Locale::en)
                        );
                        return Ok(());
                    }

                    if amount > payment_method.max_amount {
                        println!(
                            "Amount too high. Maximum amount is {} (sat)",
                            payment_method.max_amount.to_formatted_string(&Locale::en)
                        );
                        return Ok(());
                    }

                    let PostMintQuoteOnchainResponse { address, quote, .. } =
                        wallet.create_quote_onchain(amount).await?;

                    println!("Pay onchain to mint tokens:\n\n{address}");
                    quote
                }
                PaymentMethod::Bolt11 => {
                    let PostMintQuoteBolt11Response {
                        payment_request,
                        quote,
                        ..
                    } = wallet.create_quote_bolt11(amount).await?;
                    println!("Pay invoice to mint tokens:\n\n{payment_request}");
                    quote
                }
            };

            loop {
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(1_000),
                )
                .await;

                if !wallet.is_quote_paid(&payment_method, quote.clone()).await? {
                    continue;
                }

                // FIXME store quote in db and add option to retry minting later

                let mint_result = wallet
                    .mint_tokens(&payment_method, amount.into(), quote.clone())
                    .await;

                match mint_result {
                    Ok(_) => {
                        println!(
                            "Tokens minted successfully.\nNew balance {} (sat)",
                            wallet.get_balance().await?.to_formatted_string(&Locale::en)
                        );
                        break;
                    }
                    Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(_, _)) => {
                        continue;
                    }
                    Err(e) => {
                        println!("General Error: {}", e);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
