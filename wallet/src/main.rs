use std::env;

use cashurs_core::model::{BlindedMessage, Token, Tokens};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use secp256k1::SecretKey;

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
    Split { amount: u64 },
}

fn read_env() -> String {
    dotenv().expect(".env file not found");
    env::var("MINT_URL").expect("MINT_URL not found")
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

fn get_blinded_msg(blinded_messages: Vec<(BlindedMessage, SecretKey)>) -> Vec<BlindedMessage> {
    blinded_messages
        .into_iter()
        .map(|(msg, _)| msg)
        .collect::<Vec<BlindedMessage>>()
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
    //     command: Command::Split { amount: 6 },
    // };

    match cli.command {
        Command::Split {
            amount: split_amount,
        } => {
            let prompt = "Enter Token:\n\n".to_string();
            let serialized_token = wait_for_user_input(prompt);

            let tokens = Tokens::deserialize(serialized_token)?;
            // FIXME check if token is correct handle error
            let total_token_amount = tokens.get_total_amount();
            if total_token_amount < split_amount {
                println!("Not enough tokens");
                return Ok(());
            }

            print!("first_amount: {split_amount}");
            let first_secrets = wallet.create_secrets(&wallet::split_amount(split_amount));
            let first_outputs =
                wallet.create_blinded_messages(split_amount, first_secrets.clone())?;

            // ############################################################################

            let second_amount = total_token_amount - split_amount;
            print!("second_amount: {second_amount}");
            let second_secrets = wallet.create_secrets(&wallet::split_amount(second_amount));
            let second_outputs =
                wallet.create_blinded_messages(second_amount, second_secrets.clone())?;

            let mut total_outputs = vec![];
            total_outputs.extend(get_blinded_msg(first_outputs.clone()));
            total_outputs.extend(get_blinded_msg(second_outputs.clone()));

            let split_result = client
                .post_split_tokens(split_amount, tokens.get_proofs(), total_outputs)
                .await?;

            println!("split result:\n\n{split_result:?}");

            let first_proofs = wallet.create_proofs_from_blinded_signatures(
                split_result.fst,
                first_secrets,
                first_outputs,
            )?;
            let first_tokens = Tokens::from((mint_url.clone(), first_proofs));
            println!(
                "Split tokens {} sats:\n\n{:?}",
                split_amount,
                first_tokens.serialize()?
            );

            let second_proofs = wallet.create_proofs_from_blinded_signatures(
                split_result.snd,
                second_secrets,
                second_outputs,
            )?;
            let second_tokens = Tokens::from((mint_url.clone(), second_proofs));
            println!(
                "Remaining tokens {} sats:\n\n{:?}",
                second_amount,
                second_tokens.serialize()?
            );
        }
        Command::Melt { token } => {
            println!("melt tokens");
            let deserialized = Tokens::deserialize(token)?;

            let prompt = "Enter invoice:\n\n".to_string();
            let pr = wait_for_user_input(prompt);

            println!(">> {}", pr);

            let response = wallet.melt_token(pr, deserialized).await?;
            if response.paid {
                println!("Invoice has been paid: Tokens melted successfully");
                // TODO NUT-08 create tokens from change
            } else {
                println!("Tokens not melted");
            }
        }
        Command::Mint { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await?;
            let payment_hash = payment_request.clone().hash;
            let invoice = payment_request.clone().pr;

            let prompt = format!(
                "Pay invoice to mint sats. Press return after invoice is paid:\n\n{invoice}"
            );
            wait_for_user_input(prompt);

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
