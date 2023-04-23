use clap::{Parser, Subcommand};

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

fn main() {
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
}
