use std::{env, time::Duration};

use cashurs_core::model::TokenV3;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

use cashurs_wallet::localstore::SqliteLocalStore;
use cashurs_wallet::wallet;

use cashurs_wallet::client::Client;
use cashurs_wallet::localstore::LocalStore;
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

    /// Split tokens without storing them
    Split { amount: u64 },
}

fn read_env(variable: &str) -> String {
    dotenv().expect(".env file not found");
    env::var(variable).expect("MINT_URL not found")
}

fn wait_for_user_input(prompt: String) -> String {
    println!("{prompt}");
    let mut result = String::new();
    loop {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Error: Could not read a line");
        result.push_str(&line);
        if line == "\n" {
            return result.trim().to_string();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mint_url = Url::parse(&read_env("WALLET_MINT_URL"))?;

    let client = cashurs_wallet::client::HttpClient::new();
    let keys = client.get_mint_keys(&mint_url).await?;
    let keysets = client.get_mint_keysets(&mint_url).await?;

    let db_path = read_env("WALLET_DB_PATH");
    let localstore = Box::new(SqliteLocalStore::with_path(db_path).await?);
    localstore.migrate().await;

    let wallet = wallet::Wallet::new(
        Box::new(client.clone()),
        keys,
        keysets,
        localstore.clone(),
        mint_url.clone(),
    );

    let cli = Opts::parse();

    match cli.command {
        Command::Receive { token } => {
            let tokens = TokenV3::deserialize(token)?;
            wallet.receive_tokens(&tokens).await?;
            println!(
                "Tokens received successfully.\nNew balance {} sats",
                wallet.get_balance().await?
            );
        }
        Command::Send { amount } => {
            let balance = wallet.get_balance().await?;
            if amount > balance {
                println!("Not enough balance");
                return Ok(());
            }

            let selected_proofs = wallet.get_proofs_for_amount(amount).await?;
            let selected_tokens = (mint_url.as_ref().to_owned(), selected_proofs.clone()).into();

            let (remaining_tokens, result) = wallet.split_tokens(&selected_tokens, amount).await?;

            localstore.delete_proofs(&selected_proofs).await?;
            localstore.add_proofs(&remaining_tokens.proofs()).await?;

            let amount = result.total_amount();
            let ser = result.serialize()?;

            println!("Result {amount} sats:\n{ser}");
            println!("\nNew balance: {:?} sats", wallet.get_balance().await?);
        }

        Command::Balance => {
            let balance = wallet.get_balance().await?;
            println!("Balance: {balance:?} sats");
        }
        Command::Split {
            amount: splt_amount,
        } => {
            let prompt = "Enter Token:\n".to_string();
            let serialized_token = wait_for_user_input(prompt);

            let tokens = TokenV3::deserialize(serialized_token)?;
            let total_token_amount = tokens.total_amount();
            if total_token_amount < splt_amount {
                println!("Not enough tokens");
                return Ok(());
            }
            let (first_tokens, second_tokens) = wallet.split_tokens(&tokens, splt_amount).await?;

            println!(
                "\nTokens ({:?} sats):\n{}",
                second_tokens.total_amount(),
                second_tokens.serialize()?
            );
            println!(
                "\nTokens ({:?} sats):\n{}",
                first_tokens.total_amount(),
                first_tokens.serialize()?
            );
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
