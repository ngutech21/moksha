use std::env;

use cashurs_core::model::{Token, Tokens};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

mod client;
mod error;
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
}

fn read_env() -> String {
    dotenv().expect(".env file not found");
    env::var("MINT_URL").expect("MINT_URL not found")
}

fn wait_for_payment(invoice: String) {
    println!("Pay invoice to mint sats. Press return after invoice is paid:\n\n{invoice}");
    loop {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Error: Could not read a line");
        if line == "\n" {
            break;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mint_url = read_env();

    let client = client::Client::new(mint_url.clone());
    let keys = client.get_mint_keys().await?;
    let keysets = client.get_mint_keysets().await?;

    let wallet = wallet::Wallet::new(client.clone(), keys, keysets);

    let cli = Opts::parse();
    // let cli = Opts {
    //     command: Command::Mint { amount: 100 },
    // };

    match cli.command {
        Command::Melt { token } => {
            println!("melt tokens");
            let deserialized = Tokens::deserialize(token)?;
            wallet.melt_token(deserialized);
        }
        Command::Mint { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await?;
            let payment_hash = payment_request.clone().hash;

            wait_for_payment(payment_request.pr);

            let proofs = wallet.mint_tokens(amount, payment_hash).await?;

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
