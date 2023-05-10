use std::env;

use cashurs_core::model::{Token, Tokens};
use clap::{Parser, Subcommand};
use client::Client;
use dotenvy::dotenv;
use localstore::RocksDBLocalStore;

mod client;
mod error;
mod localstore;
mod wallet;

#[derive(Parser)]
#[command(version)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    Mint { amount: u64 },
    Melt { token: String },
    Split { amount: u64 },
    Balance,
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
    let mint_url = read_env("MINT_URL");

    let client = client::HttpClient::new(mint_url.clone());
    let keys = client.get_mint_keys().await?;
    let keysets = client.get_mint_keysets().await?;

    let localstore = Box::new(RocksDBLocalStore::new(read_env("WALLET_DB_PATH")));

    let wallet = wallet::Wallet::new(Box::new(client.clone()), keys, keysets, localstore);

    let cli = Opts::parse();
    // let cli = Opts {
    //     command: Command::Split { amount: 6 },
    // };

    match cli.command {
        Command::Balance => {
            let balance = wallet.get_balance()?;
            println!("Balance: {:?}", balance);
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
            let (first_tokens, second_tokens) =
                wallet.split_tokens(tokens, splt_amount, mint_url).await?;

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
        Command::Melt { token } => {
            let deserialized = Tokens::deserialize(token)?;

            let prompt = "Enter invoice:\n\n".to_string();
            let pr = wait_for_user_input(prompt);

            let response = wallet.melt_token(pr, deserialized).await?;
            if response.paid {
                println!("Invoice has been paid: Tokens melted successfully");
                // TODO NUT-08 create tokens from change
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::Mint { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await?;
            let hash = payment_request.clone().hash;
            let invoice = payment_request.clone().pr;

            let prompt = format!(
                "Pay invoice to mint tokens. Press return after invoice is paid:\n\n{invoice}"
            );
            wait_for_user_input(prompt);

            let proofs = wallet.mint_tokens(amount, hash).await?;

            let serialized_tokens = Tokens::new(Token {
                mint: Some(mint_url.to_string()),
                proofs,
            })
            .serialize()?;

            println!("Minted tokens:\n\n{serialized_tokens}");
        }
    }
    Ok(())
}
