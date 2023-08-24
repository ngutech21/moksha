use clap::{Parser, Subcommand};
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

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use moksha_wallet::localstore::{sqlite::SqliteLocalStore, LocalStore};

    let cli = Opts::parse();

    let db_path = match cli.db_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir.clone())?;
            dir.join("wallet.db").to_str().unwrap().to_string()
        }

        None => moksha_wallet::config_path::db_path(),
    };

    let localstore = SqliteLocalStore::with_path(db_path.clone()).await?;
    localstore.migrate().await;

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
                wallet.get_balance().await?
            );
        }
        Command::Send { amount } => {
            let result = wallet.send_tokens(amount).await?;
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
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(1_000),
                )
                .await;
                let mint_result = wallet.mint_tokens(amount.into(), hash.clone()).await;

                match mint_result {
                    Ok(_) => {
                        println!(
                            "Tokens minted successfully.\nNew balance {} sats",
                            wallet.get_balance().await?
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

#[cfg(target_arch = "wasm32")]
fn main() {}
