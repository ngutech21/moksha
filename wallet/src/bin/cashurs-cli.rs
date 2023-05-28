use std::{env, time::Duration};

use cashurs_core::model::Tokens;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

use cashurs_wallet::localstore::RocksDBLocalStore;
use cashurs_wallet::wallet;

use cashurs_wallet::client::Client;
use cashurs_wallet::localstore::LocalStore;
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
    let mint_url = read_env("WALLET_MINT_URL");

    let client = cashurs_wallet::client::HttpClient::new(mint_url.clone());
    let keys = client.get_mint_keys().await?;
    let keysets = client.get_mint_keysets().await?;

    let localstore = Box::new(RocksDBLocalStore::new(read_env("WALLET_DB_PATH")));

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
            let tokens = Tokens::deserialize(token)?;
            let total_amount = tokens.total_amount();
            let (_, redeemed_tokens) = wallet.split_tokens(tokens, total_amount).await?;
            localstore.add_tokens(redeemed_tokens)?;
            println!(
                "Tokens received successfully.\nNew balance {} sats",
                wallet.get_balance()?
            );
        }
        Command::Send { amount } => {
            let balance = wallet.get_balance()?;
            if amount > balance {
                println!("Not enough balance");
                return Ok(());
            }

            let selected_proofs = wallet.get_proofs_for_amount(amount)?;
            let selected_proofs = Tokens::from((mint_url.clone(), selected_proofs.clone()));

            let (remaining_tokens, result) =
                wallet.split_tokens(selected_proofs.clone(), amount).await?;

            localstore.delete_tokens(selected_proofs)?;
            localstore.add_tokens(remaining_tokens)?;

            let amount = result.total_amount();
            let ser = result.serialize()?;

            println!("Result {amount} sats:\n{ser}");
            println!("\nNew balance: {:?} sats", wallet.get_balance()?);
        }

        Command::Balance => {
            let balance = wallet.get_balance()?;
            println!("Balance: {balance:?} sats");
        }
        Command::Split {
            amount: splt_amount,
        } => {
            let prompt = "Enter Token:\n".to_string();
            let serialized_token = wait_for_user_input(prompt);

            let tokens = Tokens::deserialize(serialized_token)?;
            let total_token_amount = tokens.total_amount();
            if total_token_amount < splt_amount {
                println!("Not enough tokens");
                return Ok(());
            }
            let (first_tokens, second_tokens) = wallet.split_tokens(tokens, splt_amount).await?;

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
            let all_tokens = localstore.get_tokens()?;

            let fees = client.post_checkfees(invoice.clone()).await?;
            let ln_amount = wallet.get_invoice_amount(&invoice)? + (fees.fee / 1000);

            if ln_amount > all_tokens.total_amount() {
                println!("Not enough tokens");
                return Ok(());
            }
            let selected_proofs = wallet.get_proofs_for_amount(ln_amount)?;

            let total_proofs = if selected_proofs.get_total_amount() > ln_amount {
                let selected_tokens = Tokens::from((mint_url.clone(), selected_proofs.clone()));
                let split_result = wallet
                    .split_tokens(selected_tokens.clone(), ln_amount)
                    .await?;

                localstore.delete_tokens(selected_tokens)?;
                localstore.add_tokens(split_result.0)?;

                split_result.1.get_proofs()
            } else {
                selected_proofs
            };

            let response = wallet.melt_token(invoice, ln_amount, total_proofs).await?;

            if response.paid {
                println!(
                    "\nInvoice has been paid: Tokens melted successfully\nNew balance: {:?} sats",
                    wallet.get_balance()?
                );
                // TODO NUT-08 create tokens from change
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::Mint { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await?;
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
                            wallet.get_balance()?
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
