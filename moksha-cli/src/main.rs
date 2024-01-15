use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};
use moksha_core::primitives::{
    PaymentMethod, PostMintQuoteBolt11Response, PostMintQuoteOnchainResponse,
};
use num_format::{Locale, ToFormattedString};
use std::path::PathBuf;
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
                "Tokens received successfully.\nNew balance {} sats",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }
        Command::Send { amount } => {
            let result = wallet.send_tokens(amount).await?;
            let ser: String = result.try_into()?;

            println!("Result {amount} sats:\n{ser}");
            println!(
                "\nNew balance: {} sats",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }

        Command::Balance => {
            let balance = wallet.get_balance().await?.to_formatted_string(&Locale::en);
            println!("Balance: {balance} sats");
        }
        Command::Pay { invoice } => {
            let response = wallet.pay_invoice(invoice).await?;

            // FIXME handle not enough tokens error

            if response.paid {
                println!(
                    "\nInvoice has been paid: Tokens melted successfully\nNew balance: {} sats",
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

            let response = wallet.pay_onchain(address, amount).await?;

            if response.paid {
                println!(
                    "\nTokens melted successfully\nTransaction-id {}\nNew balance: {} sats",
                    response.txid,
                    wallet.get_balance().await?.to_formatted_string(&Locale::en)
                );
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::Mint { amount } => {
            let info = wallet.get_mint_info().await?;

            let payment_method = match info.nuts.nut14 {
                Some(nut14) => {
                    if nut14.supported {
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
                        println!("Selection: {}", selection);
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
                            "Tokens minted successfully.\nNew balance {} sats",
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
