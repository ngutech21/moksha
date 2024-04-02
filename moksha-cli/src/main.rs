use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use moksha_core::primitives::{
    CurrencyUnit, PaymentMethod, PostMeltBtcOnchainResponse, PostMintQuoteBolt11Response,
    PostMintQuoteBtcOnchainResponse,
};
use moksha_core::token::TokenV3;
use moksha_wallet::client::CashuClient;
use moksha_wallet::error::MokshaWalletError;
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::Wallet;
use num_format::{Locale, ToFormattedString};
use std::collections::HashSet;
use std::io::Write;
use std::str::FromStr;
use std::{io::stdout, path::PathBuf};
use url::Url;

#[derive(Parser)]
#[command(version)]
struct Opts {
    #[clap(short, long)]
    db_dir: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    /// Mint tokens
    Mint { amount: u64 },

    /// Pay Lightning invoice
    Pay { invoice: String },

    /// Pay Bitcoin on chain
    PayOnchain { address: String, amount: u64 },

    /// Send tokens
    Send { amount: u64 },

    /// Receive tokens
    Receive { token: String },

    /// Show local balance
    Balance,

    /// Show version and configuration
    Info,

    /// Mint tokens
    AddMint { mint_url: Url },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use moksha_wallet::localstore::sqlite::SqliteLocalStore;

    let cli = Opts::parse();

    let db_path = match cli.db_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir.clone())?;
            dir.join("wallet.db").to_str().unwrap().to_string()
        }

        None => moksha_wallet::config_path::db_path(),
    };

    let localstore = SqliteLocalStore::with_path(db_path.clone()).await?;
    let client = CrossPlatformHttpClient::new();
    let wallet = moksha_wallet::wallet::WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await
        .map_err(|e| {
            if matches!(
                e,
                moksha_wallet::error::MokshaWalletError::UnsupportedApiVersion
            ) {
                println!("Error: Mint does not support /v1 api");
                std::process::exit(1);
            }
            e
        })?;

    match cli.command {
        Command::AddMint { mint_url } => {
            wallet.add_mint_keysets(&mint_url).await?;
            println!("Mint added successfully ");
        }
        Command::Info => {
            let wallet_version = env!("CARGO_PKG_VERSION");
            println!("Version: {}\nDB: {}", wallet_version, db_path,);
        }
        // checks if the mints keyset is already in the wallet, if not it adds it and then imports the tokens
        Command::Receive { token } => {
            let token: TokenV3 = TokenV3::from_str(&token)?;
            let mint_urls = wallet.get_mint_urls().await?;

            let token_mint_url = match token.mint() {
                Some(url) => url,
                None => {
                    println!("Invalid Token: Missing mint url");
                    return Ok(());
                }
            };

            if !mint_urls.contains(&token_mint_url) {
                let add_mint = Confirm::new()
                    .with_prompt(format!(
                        "New mint found {:?} . Do you want to add it?",
                        token_mint_url.to_string()
                    ))
                    .interact()
                    .unwrap();

                if !add_mint {
                    return Ok(());
                }

                let is_valid_mint = CrossPlatformHttpClient::new()
                    .is_v1_supported(&token_mint_url)
                    .await?;
                if !is_valid_mint {
                    println!("Error: Invalid mint url");
                    return Ok(());
                }

                wallet.add_mint_keysets(&token_mint_url).await?;
            }

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .iter()
                .find(|k| k.mint_url == token_mint_url)
                .expect("Keyset not found");

            wallet.receive_tokens(wallet_keyset, &token).await?;
            println!(
                "Tokens received successfully.\nNew balance {} (sat)",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }
        Command::Send { amount } => {
            let mint_url = choose_mint_url(&wallet).await?;
            let mint_url = match mint_url {
                Some(url) => url,
                None => {
                    println!("No mints found.");
                    return Ok(());
                }
            };

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .iter()
                .find(|k| k.mint_url == mint_url)
                .expect("Keyset not found");

            println!("Using tokens from mint: {mint_url}");
            let result = wallet.send_tokens(wallet_keyset, amount).await?;
            let ser: String = result.try_into()?;

            println!("Result {amount} (sat):\n{ser}");
            println!(
                "\nNew balance: {} (sat)",
                wallet.get_balance().await?.to_formatted_string(&Locale::en)
            );
        }
        Command::Balance => {
            let balance = wallet.get_balance().await?.to_formatted_string(&Locale::en);
            println!("Balance: {balance} (sat)");
        }
        Command::Pay { invoice } => {
            let mint_url = choose_mint_url(&wallet).await?;
            let mint_url = match mint_url {
                Some(url) => url,
                None => {
                    println!("No mints found.");
                    return Ok(());
                }
            };
            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .iter()
                .find(|k| k.mint_url == mint_url)
                .expect("Keyset not found");

            let quote = wallet
                .get_melt_quote_bolt11(&mint_url, invoice.clone(), CurrencyUnit::Sat)
                .await?;

            let pay_confirmed = Confirm::new()
                .with_prompt(format!(
                    "Pay lightning invoice: amount {} + fee {} = {} (sat)?",
                    quote.amount,
                    quote.fee_reserve,
                    quote.amount + quote.fee_reserve
                ))
                .interact()
                .unwrap();

            if !pay_confirmed {
                return Ok(());
            }

            let response = wallet.pay_invoice(wallet_keyset, &quote, invoice).await?;

            // FIXME handle not enough tokens error

            if response.0.paid {
                if response.1 > 0 {
                    println!(
                        "Returned fees {} (sat)",
                        response.1.to_formatted_string(&Locale::en)
                    );
                }
                println!(
                    "\nInvoice has been paid: Tokens melted successfully\nNew balance: {} (sat)",
                    wallet.get_balance().await?.to_formatted_string(&Locale::en)
                );
            } else {
                println!("Error: Tokens not melted");
            }
        }
        Command::PayOnchain { address, amount } => {
            // FIXME remove redundant code
            let mint_url = choose_mint_url(&wallet).await?;
            let mint_url = match mint_url {
                Some(url) => url,
                None => {
                    println!("No mints found.");
                    return Ok(());
                }
            };
            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .iter()
                .find(|k| k.mint_url == mint_url)
                .expect("Keyset not found");

            let info = wallet.get_mint_info(&mint_url).await?;

            if info.nuts.nut18.map_or(true, |nut18| !nut18.supported) {
                println!("Error: onchain-payments are not supported by this mint");
                return Ok(());
            }

            let quotes = wallet
                .get_melt_quote_btconchain(&mint_url, address.clone(), amount)
                .await?;

            if quotes.is_empty() {
                println!("Error: No quotes found");
                return Ok(());
            }

            let quote = quotes.first().expect("No quotes found");

            println!(
                "Create onchain transaction to melt tokens: amount {} + fee {} = {} (sat)\n\n{}",
                amount,
                quote.fee,
                amount + quote.fee,
                address
            );
            let pay_confirmed = Confirm::new()
                .with_prompt("Confirm payment?")
                .interact()
                .unwrap();

            if !pay_confirmed {
                return Ok(());
            }

            let PostMeltBtcOnchainResponse { paid, txid } =
                wallet.pay_onchain(wallet_keyset, quote).await?;
            println!("Created transaction: {}\n", &txid);

            let mut lock = stdout().lock();
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;

                if paid || wallet.is_onchain_tx_paid(&mint_url, txid.clone()).await? {
                    println!(
                        "\nTokens melted successfully\nNew balance: {} (sat)",
                        wallet.get_balance().await?.to_formatted_string(&Locale::en)
                    );
                    break;
                } else {
                    write!(lock, ".").unwrap();
                    lock.flush().unwrap();
                    continue;
                }
            }
        }
        Command::Mint { amount } => {
            let mint_url = choose_mint_url(&wallet).await?;
            let mint_url = match mint_url {
                Some(url) => url,
                None => {
                    println!("No mints found.");
                    return Ok(());
                }
            };

            let info = wallet.get_mint_info(&mint_url).await?;

            let payment_method = info.nuts.nut17.as_ref().map_or_else(
                || {
                    println!("Only bolt11 minting is supported");
                    PaymentMethod::Bolt11
                },
                |nut17| {
                    if !nut17.supported {
                        println!("Only bolt11 minting is supported");
                        PaymentMethod::Bolt11
                    } else {
                        let selections = &[PaymentMethod::BtcOnchain, PaymentMethod::Bolt11];

                        let selection = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose a payment method:")
                            .default(0)
                            .items(&selections[..])
                            .interact()
                            .unwrap();
                        selections[selection].clone()
                    }
                },
            );

            let quote = match payment_method {
                PaymentMethod::BtcOnchain => {
                    let nut17 = info.nuts.nut17.expect("nut17 is None");
                    let payment_method = nut17.payment_methods.first().expect("no payment methods");
                    if amount < payment_method.min_amount {
                        println!(
                            "Amount too low. Minimum amount is {} (sat)",
                            payment_method.min_amount.to_formatted_string(&Locale::en)
                        );
                        return Ok(());
                    }

                    if amount > payment_method.max_amount {
                        println!(
                            "Amount too high. Maximum amount is {} (sat)",
                            payment_method.max_amount.to_formatted_string(&Locale::en)
                        );
                        return Ok(());
                    }

                    let PostMintQuoteBtcOnchainResponse { address, quote, .. } =
                        wallet.create_quote_onchain(&mint_url, amount).await?;

                    println!("Pay onchain to mint tokens:\n\n{address}");
                    quote
                }
                PaymentMethod::Bolt11 => {
                    let PostMintQuoteBolt11Response {
                        payment_request,
                        quote,
                        ..
                    } = wallet.create_quote_bolt11(&mint_url, amount).await?;
                    println!("Pay invoice to mint tokens:\n\n{payment_request}");
                    quote
                }
            };

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets
                .iter()
                .find(|k| k.mint_url == mint_url)
                .expect("Keyset not found");

            loop {
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(1_000),
                )
                .await;

                if !wallet
                    .is_quote_paid(&mint_url, &payment_method, quote.clone())
                    .await?
                {
                    continue;
                }

                // FIXME store quote in db and add option to retry minting later

                let mint_result = wallet
                    .mint_tokens(wallet_keyset, &payment_method, amount.into(), quote.clone())
                    .await;

                match mint_result {
                    Ok(_) => {
                        println!(
                            "Tokens minted successfully.\nNew balance {} (sat)",
                            wallet.get_balance().await?.to_formatted_string(&Locale::en)
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

pub async fn choose_mint_url(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
) -> Result<Option<Url>, MokshaWalletError> {
    let keysets = wallet.get_wallet_keysets().await?;
    if keysets.is_empty() {
        return Ok(None);
    }
    let mints: HashSet<Url> = keysets.into_iter().map(|k| k.mint_url).collect();
    let mints: Vec<Url> = mints.into_iter().collect();

    if mints.len() == 1 {
        return Ok(Some(mints.first().expect("mint not found").clone()));
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a mint:")
        .default(0)
        .items(&mints[..])
        .interact()
        .unwrap();
    let url = mints[selection].clone();
    Ok(Some(url))
}
