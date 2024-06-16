use crate::diagnostics::{apply_diagnostic, snapshot_read_diagnostic};

pub fn snapshot(output: Option<std::path::PathBuf>) -> miette::Result<()> {
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

    Ok(())
}

pub fn rebuild() -> miette::Result<()> {
    let Err(why) = crate::tasks::block_single(twon_persistence::ops::build::rebuild()) else {
        return Ok(());
    };

    match why {
        twon_persistence::ops::build::Error::Write => {
            let diagnostic = miette::diagnostic!(
                severity = miette::Severity::Error,
                code = "io::Error",
                "Failed to write snapshot",
            );
            Err(diagnostic.into())
        }
        twon_persistence::ops::build::Error::Database => {
            panic!("Failed to connect to database")
        }
        twon_persistence::ops::build::Error::SnapshotApply(error) => {
            let diagnostic = apply_diagnostic(error);
            Err(diagnostic.into())
        }
    }
}

pub fn sync() -> miette::Result<()> {
    let Err(why) = crate::tasks::block_single(twon_persistence::ops::sync::sync()) else {
        return Ok(());
    };

    match why {
        twon_persistence::ops::sync::Error::Write => {
            let diagnostic = miette::diagnostic!(
                severity = miette::Severity::Error,
                code = "io::Error",
                "Failed to write snapshot",
            );
            Err(diagnostic.into())
        }
        twon_persistence::ops::sync::Error::Database => {
            panic!("Failed to connect to database")
        }
        twon_persistence::ops::sync::Error::SnapshotApply(error) => {
            let diagnostic = apply_diagnostic(error);
            Err(diagnostic.into())
        }
        twon_persistence::ops::sync::Error::Read(error) => {
            let diagnostic = snapshot_read_diagnostic(error);
            Err(diagnostic)
        }
    }
}

pub mod actions {
    #[derive(clap::Subcommand)]
    pub enum ActionCommand {
        CreateWallet {
            #[arg(short, long)]
            currency_id: u32,
            #[arg(short, long)]
            name: Option<String>,
        },
    }

    pub fn create_wallet(currency_id: u32, name: Option<String>) -> miette::Result<()> {
        let wallet_id = crate::tasks::block_single(async move {
            let db = twon_persistence::database::connect().await.unwrap();
            let result = twon_persistence::actions::create_wallet::run(
                &db,
                twon_core::CurrencyId::new(currency_id),
                name,
            )
            .await;

            match result {
                Ok(wallet_id) => wallet_id,
                Err(e) => panic!("Failed to create wallet {e:?}"),
            }
        });

        println!("Wallet `{}` created", wallet_id);
        Ok(())
    }
}

pub mod event {
    use twon_core::{Amount, CurrencyId, WalletId};

    use crate::diagnostics::{apply_diagnostic, snapshot_read_diagnostic};

    #[derive(clap::Subcommand)]
    pub enum EventCommand {
        Add {
            #[command(subcommand)]
            commands: AddEvent,
        },
    }

    #[derive(clap::Subcommand)]
    pub enum AddEvent {
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

    pub fn add_event(command: AddEvent) -> miette::Result<()> {
        let mut snapshot_io = twon_persistence::SnapshotIO::new();
        let mut snapshot_entry = match snapshot_io.read() {
            Ok(snapshot_entry) => snapshot_entry,
            Err(why) => return Err(snapshot_read_diagnostic(why)),
        };

        let event = match command {
            AddEvent::Deposit { wallet_id, amount } => {
                twon_core::Event::Deposit { amount, wallet_id }
            }
            AddEvent::Deduct { wallet_id, amount } => {
                twon_core::Event::Deduct { amount, wallet_id }
            }
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

        crate::tasks::block_single(async {
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
}
