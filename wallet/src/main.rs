use std::env;

use cashurs_core::{
    dhke,
    model::{BlindedMessage, Proof, Token, Tokens},
};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use rand::{distributions::Alphanumeric, Rng};

mod client;

#[derive(Parser)]
#[command(version)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    Balance,
    Invoice { amount: u64 },
    Send { amount: u64 },
    Pay { invoice: String },
    Info,
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

fn read_env() -> String {
    dotenv().expect(".env file not found");
    env::var("MINT_URL").expect("MINT_URL not found")
}

fn wait_for_payment(invoice: String) {
    println!(">> press return after invoice is paid: {invoice:?}");
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

/// split a decimal amount into a vector of powers of 2
fn amount_split(amount: u64) -> Vec<u64> {
    format!("{:b}", amount)
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '1' {
                return Some(2_u64.pow(i as u32));
            }
            None
        })
        .collect::<Vec<u64>>()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    generate_random_string();
    println!("start");
    let mint_url = read_env();

    let client = client::Client::new(mint_url.clone());
    let keys = client.get_mint_keys().await.unwrap();
    let keysets = client.get_mint_keysets().await;

    let cli = Opts::parse();

    match cli.command {
        Command::Invoice { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await;
            let payment_hash = payment_request.clone().unwrap().hash;

            let invoice = payment_request.unwrap().pr;

            println!(">> invoice: {payment_hash:?}");
            wait_for_payment(invoice);
            println!(">> invoice paid");
            let token_secret = generate_random_string();

            let (b_, alice_secret_key) = dhke::step1_alice(token_secret.clone(), None).unwrap();

            // FIXME use split_amount
            let msg = BlindedMessage { amount: 2, b_ };
            let post_mint_resp = client
                .post_mint_payment_request(payment_hash, vec![msg])
                .await
                .unwrap();

            // step 3: unblind signatures
            //println!("Send {amount} {payment_request:?} {post_mint_resp:?}");
            let c_ = post_mint_resp.promises[0].c_;
            let key = keys.get(&2).unwrap();
            let pub_alice = dhke::step3_alice(c_, alice_secret_key, *key);

            let keysets = keysets.unwrap().keysets;

            let proof = Proof::new(
                post_mint_resp.promises[0].amount,
                token_secret.to_string(),
                pub_alice,
                keysets[1].clone(), // FIXME choose correct keyset
            );

            let token = Token {
                mint: Some(mint_url.to_string()),
                proofs: vec![proof],
            };

            let tokens = Tokens {
                memo: Some("my memo".to_string()),
                tokens: vec![token],
            };

            let serialized_tokens = tokens.serialize();
            println!("minted tokens {:?}", serialized_tokens.unwrap());
        }
        Command::Pay { invoice } => {
            println!("Pay {invoice}");
        }
        Command::Info => {
            println!("Info");
        }
        Command::Balance => {
            println!("Balance");
        }
        Command::Send { amount } => {
            println!("Send {amount}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_split() -> anyhow::Result<()> {
        let amount = 13;
        let bits = super::amount_split(amount);
        assert_eq!(bits, vec![1, 4, 8]);
        Ok(())
    }
}
