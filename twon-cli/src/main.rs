use clap::Parser;
use twon_core::{Balance, CurrencyId, WalletId};

#[derive(clap::Parser)]
struct CliParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Events {
        #[command(subcommand)]
        commands: EventCommand,
    },
}

#[derive(clap::Subcommand)]
enum EventCommand {
    Add {
        #[command(subcommand)]
        commands: AddEvent,
    },
}

#[derive(clap::Subcommand)]
enum AddEvent {
    Deposit {
        #[arg(short, long)]
        wallet_id: WalletId,
        #[arg(short, long)]
        amount: Balance,
    },
    CreateWallet {
        #[arg(short, long)]
        wallet_id: WalletId,
        #[arg(short, long)]
        currency_id: u32,
    },
}

fn main() {
    let cli = CliParser::parse();
    match cli.command {
        Commands::Events { commands } => match commands {
            EventCommand::Add { commands } => {
                let mut snapshot_io = twon_persistence::SnapshotIO::new();
                let mut snapshot_entry = snapshot_io.read().expect("Failed to read snapshot");

                let event = match commands {
                    AddEvent::Deposit { wallet_id, amount } => twon_core::Event::Deposit {
                        amount,
                        wallet_id,
                    },
                    AddEvent::CreateWallet {
                        wallet_id,
                        currency_id,
                    } => twon_core::Event::CreateWallet {
                        wallet_id,
                        currency: CurrencyId::new(currency_id),
                    },
                };

                if let Err(why) = snapshot_entry.snapshot.apply(event.clone()) {
                    panic!("{:?}", why);
                };

                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime")
                    .block_on(async {
                        let db = twon_persistence::database::connect()
                            .await
                            .expect("Failed to connect");

                        twon_persistence::database::add_event(&db, event, 0)
                            .await
                            .expect("Failed to add event");
                    });

                snapshot_io
                    .write(snapshot_entry)
                    .expect("Failed to write snapshot");
            }
        },
    }
}
