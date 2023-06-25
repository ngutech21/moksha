use std::{env, time::Duration};

use cashurs_wallet::localstore::LocalStore;
use cashurs_wallet::{localstore::SqliteLocalStore, wallet::Wallet};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use reqwest::Url;
use tokio::time::{sleep_until, Instant};

#[derive(Parser)]
#[command(version)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    /// Mint tokens
    Mint { amount: u64 },

    /// Pay Lightning invoice
    Pay { invoice: String },

    /// Send tokens
    Send { amount: u64 },

    /// Receive tokens
    Receive { token: String },

    /// Show local balance
    Balance,
}

fn read_env(variable: &str) -> String {
    dotenv().expect(".env file not found"); // FIXME remove dotenv-check
    env::var(variable).expect("MINT_URL not found")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = Wallet::db_path();
    println!("Using db: {}", db_path);
    let localstore = Box::new(SqliteLocalStore::with_path(db_path).await?);
    localstore.migrate().await;

    let mint_url = Url::parse(&read_env("WALLET_MINT_URL"))?;
    let client = Box::new(cashurs_wallet::client::HttpClient::new());

    let wallet = Wallet::builder()
        .with_client(client)
        .with_localstore(localstore)
        .with_mint_url(mint_url)
        .build()
        .await?;

    let cli = Opts::parse();

    match cli.command {
        Command::Receive { token } => {
            wallet.receive_tokens(&token.try_into()?).await?;
            println!(
                "Tokens received successfully.\nNew balance {} sats",
                wallet.get_balance().await?
            );
        }
        Command::Send { amount } => {
            let result = wallet.send_tokens(amount).await?;
            // FIXME handle error not enough tokens

            // let balance = wallet.get_balance().await?;
            // if amount > balance {
            //     println!("Not enough balance");
            //     return Ok(());
            // }
            let ser: String = result.try_into()?;

            println!("Result {amount} sats:\n{ser}");
            println!("\nNew balance: {:?} sats", wallet.get_balance().await?);
        }

        Command::Balance => {
            let balance = wallet.get_balance().await?;
            println!("Balance: {balance:?} sats");
        }
        Command::Pay { invoice } => {
            let response = wallet.pay_invoice(invoice).await?;

            // FIXME handle not enough tokens error

            if response.paid {
                println!(
                    "\nInvoice has been paid: Tokens melted successfully\nNew balance: {:?} sats",
                    wallet.get_balance().await?
                );
                // TODO NUT-08 create tokens from change
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::Mint { amount } => {
            let payment_request = wallet.get_mint_payment_request(amount).await?;
            let hash = payment_request.clone().hash;
            let invoice = payment_request.clone().pr;

            println!("Pay invoice to mint tokens:\n\n{invoice}");

            loop {
                sleep_until(Instant::now() + Duration::from_millis(1_000)).await;
                let mint_result = wallet.mint_tokens(amount, hash.clone()).await;

                match mint_result {
                    Ok(_) => {
                        println!(
                            "Tokens minted successfully.\nNew balance {} sats",
                            wallet.get_balance().await?
                        );
                        break;
                    }
                    Err(cashurs_wallet::error::CashuWalletError::InvoiceNotPaidYet(_, _)) => {
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
