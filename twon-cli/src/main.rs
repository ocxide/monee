mod commands;
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
}

use clap::Parser;

#[derive(clap::Parser)]
struct CliParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Events {
        #[command(subcommand)]
        commands: commands::event::EventCommand,
    },
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
}

fn main() -> miette::Result<()> {
    let cli = CliParser::parse();
    match cli.command {
        Commands::Events { commands } => match commands {
            commands::event::EventCommand::Add { commands } => {
                commands::event::add_event(commands)?;
            }
        },
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
            commands::wallets::WalletCommand::Create { currency: currency_id, name } => {
                commands::wallets::create(currency_id, name)?;
            }
            commands::wallets::WalletCommand::List => {
                commands::wallets::list()?;
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
    }

    Ok(())
}
