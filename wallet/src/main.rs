use std::env;

use cashurs_core::{
    dhke,
    model::{BlindedMessage, Proof, Proofs, Token},
};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");
    let mint_url = env::var("MINT_URL");
    print!("{:?}", mint_url);

    let client = client::Client::new(mint_url.unwrap());
    let keys = client.get_mint_keys().await;
    let keysets = client.get_mint_keysets().await;

    println!("{:?}", keys);
    println!("{:?}", keysets);

    let cli = Opts::parse();

    match cli.command {
        Command::Invoice { amount } => {
            let payment_request = client.get_mint_payment_request(amount).await;
            let payment_hash = payment_request.clone().unwrap().hash;

            let my_secret = "my secret".to_string();
            let (b_, alice_secret_key) = dhke::step1_alice(my_secret, None).unwrap();

            let msg = BlindedMessage {
                amount: 1,
                b_: b_.to_string(),
            };
            let blinded_messages = vec![msg];
            let post_mint_resp = client
                .post_mint_payment_request(payment_hash, blinded_messages)
                .await
                .unwrap();

            // unblind signatures
            println!("Send {amount} {payment_request:?} {post_mint_resp:?}");

            let pubs = dhke::public_key_from_hex(&post_mint_resp.promises[0].c_);

            dhke::step3_alice(pubs, alice_secret_key, b_);

            let proof = Proof {
                amount: post_mint_resp.promises[0].amount,
                c: pubs.to_string(),
                secret: "my secret".to_string(),
                id: Some(keysets.unwrap().keysets[0].clone()),
                script: None,
            };

            let token = Token {
                mint: Some("my mint".to_string()), // FIXME
                proofs: vec![proof],
            };

            println!("token {:?}", token);
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
