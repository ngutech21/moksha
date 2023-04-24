use std::env;

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
            println!("Send {amount}");
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
