use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use moksha_core::primitives::{
    PaymentMethod, PostMeltOnchainResponse, PostMintQuoteBolt11Response,
    PostMintQuoteOnchainResponse,
};
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
    Mint {
        amount: u64,
    },

    /// Pay Lightning invoice
    Pay {
        invoice: String,
    },

    /// Pay bitcoin onchain
    PayOnchain {
        address: String,
        amount: u64,
    },

    /// Send tokens
    Send {
        amount: u64,
    },

    /// Receive tokens
    Receive {
        token: String,
    },

    /// Show local balance
    Balance,

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

    let client = moksha_wallet::client::reqwest::HttpClient::new();

    let wallet = moksha_wallet::wallet::WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .with_mint_url(cli.mint_url.clone())
        .build()
        .await?;

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
            let response = wallet.pay_invoice(invoice).await?;

            // FIXME handle not enough tokens error

            if response.paid {
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

            let quote = wallet.pay_onchain_quote(address.clone(), amount).await?;

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

            let PostMeltOnchainResponse { paid, txid } = wallet.pay_onchain(&quote).await?;
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

            let payment_method = match info.nuts.nut14 {
                Some(ref nut14) => {
                    if !nut14.supported {
                        println!("Only bolt11 minting is supported");
                        PaymentMethod::Bolt11
                    } else {
                        let selections = &[PaymentMethod::Onchain, PaymentMethod::Bolt11];

                        let selection = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose a payment method:")
                            .default(0)
                            .items(&selections[..])
                            .interact()
                            .unwrap();
                        selections[selection].clone()
                    }
                }
                None => {
                    println!("Only bolt11 minting is supported");
                    PaymentMethod::Bolt11
                }
            };

            let quote = match payment_method {
                PaymentMethod::Onchain => {
                    let nut14 = info.nuts.nut14.expect("nut14 is None");
                    if amount < nut14.min_amount {
                        println!(
                            "Amount too low. Minimum amount is {} (sat)",
                            nut14.min_amount.to_formatted_string(&Locale::en)
                        );
                        return Ok(());
                    }

                    if amount > nut14.max_amount {
                        println!(
                            "Amount too high. Maximum amount is {} (sat)",
                            nut14.max_amount.to_formatted_string(&Locale::en)
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
