use std::{process::exit, time::Duration};

use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::{ProgressBar, ProgressStyle};
use moksha_core::keyset::KeysetIdType;
use moksha_wallet::{error::MokshaWalletError, http::CrossPlatformHttpClient, localstore::sqlite::SqliteLocalStore, wallet::Wallet};
use num_format::Locale;
use url::Url;
use num_format::ToFormattedString;

pub fn progress_bar() -> anyhow::Result<ProgressBar>{
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}")?);
    Ok(pb)
            
}


pub async fn choose_mint(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
    keysetid_type: KeysetIdType,
) -> Result<(Url, u64), MokshaWalletError> {
    let mints = get_mints_with_balance(wallet, keysetid_type).await?;

    if mints.is_empty() {
        println!("No mints found. Add a mint first with 'moksha-cli add-mint <mint-url>'");
        exit(0)
    }

    if mints.len() == 1 {
        return Ok(mints[0].clone());
    }

    let mints_display = mints
        .iter()
        .map(|(url, balance)| {
            format!(
                "{} - {} (sat)",
                url,
                balance.to_formatted_string(&Locale::en)
            )
        })
        .collect::<Vec<String>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a mint:")
        .default(0)
        .items(&mints_display[..])
        .interact()
        .unwrap();
    Ok(mints[selection].clone())
}



pub async fn get_mints_with_balance(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
    keysetid_type: KeysetIdType,
) -> Result<Vec<(Url, u64)>, MokshaWalletError> {
    let all_proofs = wallet.get_proofs().await?;

    
    let keysets = wallet.get_wallet_keysets().await?;
    if keysets.is_empty() {
        println!("No mints found. Add a mint first with 'moksha-cli add-mint <mint-url>'");
        exit(0)
    }
    Ok(keysets
        .into_iter()
        .filter(|k| k.keyset_id.keyset_type() == keysetid_type)
        .map(|k| {
            (
                k.mint_url,
                all_proofs.proofs_by_keyset(&k.keyset_id).total_amount(),
            )
        })
        .collect::<Vec<(Url, u64)>>())
}


pub async fn show_total_balance(wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>) -> anyhow::Result<()>{
    let term = Term::stdout();
    term.write_line(&format!(
                "Tokens received successfully.\nNew total balance {} (sat)",
                style(wallet.get_balance().await?.to_formatted_string(&Locale::en)).cyan() )
            )?;
    Ok(())
}