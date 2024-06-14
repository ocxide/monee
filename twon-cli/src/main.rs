use clap::Parser;
use twon_core::{Amount, CurrencyId, WalletId};

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
        amount: Amount,
    },
    CreateWallet {
        #[arg(short, long)]
        wallet_id: WalletId,
        #[arg(short, long)]
        currency_id: u32,
    },
}

fn main() -> miette::Result<()> {
    let cli = CliParser::parse();
    match cli.command {
        Commands::Events { commands } => match commands {
            EventCommand::Add { commands } => {
                add_event(commands)?;
            }
        },
    }

    Ok(())
}

mod json_diagnostic {
    use twon_persistence::snapshot_io;

    #[derive(miette::Diagnostic, Debug)]
    #[diagnostic(code = "snapshot::DecodeError", severity(Error))]
    pub struct JsonDecodeDiagnostic {
        error: serde_json::Error,
        #[source_code]
        source: miette::NamedSource<String>,
        #[label = "{error}"]
        label: (usize, usize),
    }

    impl std::fmt::Display for JsonDecodeDiagnostic {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Failed to decode snapshot")
        }
    }

    impl std::error::Error for JsonDecodeDiagnostic {}

    impl From<snapshot_io::read::JsonDecodeError> for JsonDecodeDiagnostic {
        fn from(err: snapshot_io::read::JsonDecodeError) -> Self {
            let snapshot_io::read::JsonDecodeError {
                error,
                json,
                filename,
            } = err;

            let start = json
                .lines()
                .take(error.line() - 1)
                .map(|line| line.chars().count())
                .sum::<usize>()
                + error.column()
                - 1;

            let label = (start, start);

            dbg!(&label);

            Self {
                error,
                source: miette::NamedSource::new(filename.to_string_lossy(), json),
                label,
            }
        }
    }
}

fn add_event(command: AddEvent) -> miette::Result<()> {
    use twon_persistence::snapshot_io;

    let mut snapshot_io = twon_persistence::SnapshotIO::new();
    let mut snapshot_entry = match snapshot_io.read() {
        Ok(snapshot_entry) => snapshot_entry,
        Err(why) => {
            let diagnostic = match why {
                snapshot_io::read::Error::Io(err) => miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "io::ReadError",
                    "{err}",
                ),
                snapshot_io::read::Error::Json(err) => {
                    let diagnostic: json_diagnostic::JsonDecodeDiagnostic = err.into();
                    return Err(diagnostic.into());
                }
            };

            return Err(diagnostic.into());
        }
    };

    let event = match command {
        AddEvent::Deposit { wallet_id, amount } => twon_core::Event::Deposit { amount, wallet_id },
        AddEvent::CreateWallet {
            wallet_id,
            currency_id,
        } => twon_core::Event::CreateWallet {
            wallet_id,
            currency: CurrencyId::new(currency_id),
        },
    };

    if let Err(why) = snapshot_entry.snapshot.apply(event.clone()) {
        let diagnostic = miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "event::ApplyError",
            "{why:?}",
        );
        return Err(diagnostic.into());
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async {
            let db = twon_persistence::database::connect()
                .await
                .expect("Failed to connect");

            twon_persistence::database::add_event(&db, event)
                .await
                .expect("Failed to add event");
        });

    snapshot_io
        .write(snapshot_entry)
        .expect("Failed to write snapshot");

    Ok(())
}
