mod args;
mod commands;
mod date;
mod diagnostics;
mod json_diagnostic;
mod tasks {
    use std::future::Future;

    /// Blocks a single thread
    /// Do persistent operation such as write to db and filesystem
    /// Enables IO & Clock
    pub fn block_single<F: Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(fut)
    }

    pub fn block_multi<F: Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(fut)
    }

    pub async fn use_db() -> twon::database::Connection {
        match twon::database::connect().await {
            Ok(conn) => conn,
            Err(e) => twon::log::database(e),
        }
    }
}

use clap::Parser;

#[derive(clap::Parser)]
struct CliParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Snapshot {
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    Rebuild,
    Sync,
    Wallets {
        #[command(subcommand)]
        command: commands::wallets::WalletCommand,
    },
    Currencies {
        #[command(subcommand)]
        command: commands::currencies::CurrencyCommand,
    },
    Actors {
        #[command(subcommand)]
        command: commands::actors::ActorsCommand,
    },
    Debts {
        #[command(subcommand)]
        command: commands::debts::DebtsCommand,
    },
    Do(crate::commands::do_command::DoCommand),
}

fn main() -> miette::Result<()> {
    let cli = CliParser::parse();
    match cli.command {
        Commands::Snapshot { output } => {
            commands::snapshot(output)?;
        }
        Commands::Rebuild => {
            commands::rebuild()?;
        }
        Commands::Sync => {
            commands::sync()?;
        }
        Commands::Wallets { command } => match command {
            commands::wallets::WalletCommand::Create {
                currency: currency_id,
                name,
                yes,
            } => {
                commands::wallets::create(currency_id, name, yes)?;
            }
            commands::wallets::WalletCommand::List => {
                commands::wallets::list()?;
            }
            commands::wallets::WalletCommand::Deduct { wallet_id, amount } => {
                commands::wallets::deduct(wallet_id, amount)?;
            }
            commands::wallets::WalletCommand::Deposit { wallet_id, amount } => {
                commands::wallets::deposit(wallet_id, amount)?;
            }
        },
        Commands::Currencies { command } => match command {
            commands::currencies::CurrencyCommand::List => {
                commands::currencies::list()?;
            }
            commands::currencies::CurrencyCommand::Create { name, symbol, code } => {
                commands::currencies::create(name, symbol, code)?;
            }
        },
        Commands::Debts { command } => {
            commands::debts::handle(command)?;
        }
        Commands::Do(command) => {
            crate::commands::do_command::handle(command)?;
        }
        Commands::Actors { command } => {
            commands::actors::handle(command)?;
        }
    }

    Ok(())
}
