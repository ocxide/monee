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
    Snapshot {
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    Rebuild,
    Sync,
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
    Deduct {
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
    DeleteWallet {
        #[arg(short, long)]
        wallet_id: WalletId,
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
        Commands::Snapshot { output } => {
            let snapshot_entry = {
                let mut snapshot_io = twon_persistence::SnapshotIO::new();
                snapshot_io.read().map_err(snapshot_read_diagnostic)?
            };

            match output {
                Some(path) => {
                    let Ok(mut file) = std::fs::File::create(&path) else {
                        let diagnostic = miette::diagnostic!(
                            severity = miette::Severity::Error,
                            code = "io::Error",
                            "Failed to create/open file: {}",
                            path.display(),
                        );

                        return Err(diagnostic.into());
                    };

                    serde_json::to_writer(&mut file, &snapshot_entry.snapshot)
                        .expect("Failed to write snapshot");
                }
                None => {
                    serde_json::to_writer(std::io::stdout(), &snapshot_entry.snapshot)
                        .expect("Failed to write snapshot");
                }
            }
        }
        Commands::Rebuild => {
            let Err(why) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime")
                .block_on(twon_persistence::ops::build::rebuild())
            else {
                return Ok(());
            };

            match why {
                twon_persistence::ops::build::Error::Write => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "io::Error",
                        "Failed to write snapshot",
                    );
                    return Err(diagnostic.into());
                }
                twon_persistence::ops::build::Error::Database => {
                    panic!("Failed to connect to database")
                }
                twon_persistence::ops::build::Error::SnapshotApply(error) => {
                    let diagnostic = apply_diagnostic(error);
                    return Err(diagnostic.into());
                }
            }
        },
        Commands::Sync => {
            let Err(why) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime")
                .block_on(twon_persistence::ops::sync::sync())
            else {
                return Ok(());
            };

            match why {
                twon_persistence::ops::sync::Error::Write => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "io::Error",
                        "Failed to write snapshot",
                    );
                    return Err(diagnostic.into());
                }
                twon_persistence::ops::sync::Error::Database => {
                    panic!("Failed to connect to database")
                }
                twon_persistence::ops::sync::Error::SnapshotApply(error) => {
                    let diagnostic = apply_diagnostic(error);
                    return Err(diagnostic.into());
                }
                twon_persistence::ops::sync::Error::Read(error) => {
                    let diagnostic = snapshot_read_diagnostic(error);
                    return Err(diagnostic);
                }
            }
        }
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

fn snapshot_read_diagnostic(error: twon_persistence::snapshot_io::read::Error) -> miette::Report {
    use twon_persistence::snapshot_io;

    match error {
        snapshot_io::read::Error::Io(err) => miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "io::ReadError",
            "{err}",
        )
        .into(),
        snapshot_io::read::Error::Json(err) => {
            let diagnostic: json_diagnostic::JsonDecodeDiagnostic = err.into();
            diagnostic.into()
        }
    }
}

fn apply_diagnostic(why: twon_core::Error) -> miette::MietteDiagnostic {
    let diagnostic = miette::diagnostic!(
        severity = miette::Severity::Error,
        code = "event::ApplyError",
        "{why:?}",
    );
    diagnostic
}

fn add_event(command: AddEvent) -> miette::Result<()> {
    let mut snapshot_io = twon_persistence::SnapshotIO::new();
    let mut snapshot_entry = match snapshot_io.read() {
        Ok(snapshot_entry) => snapshot_entry,
        Err(why) => return Err(snapshot_read_diagnostic(why)),
    };

    let event = match command {
        AddEvent::Deposit { wallet_id, amount } => twon_core::Event::Deposit { amount, wallet_id },
        AddEvent::Deduct { wallet_id, amount } => twon_core::Event::Deduct { amount, wallet_id },
        AddEvent::DeleteWallet { wallet_id } => twon_core::Event::DeleteWallet { wallet_id },
        AddEvent::CreateWallet {
            wallet_id,
            currency_id,
        } => twon_core::Event::CreateWallet {
            wallet_id,
            currency: CurrencyId::new(currency_id),
        },
    };

    if let Err(why) = snapshot_entry.snapshot.apply(event.clone()) {
        let diagnostic = apply_diagnostic(why);
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
        .write(snapshot_entry.snapshot)
        .expect("Failed to write snapshot");

    Ok(())
}
