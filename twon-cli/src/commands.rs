use crate::diagnostics::snapshot_read_diagnostic;

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

    Err(crate::diagnostics::snapshot_write_diagnostic(why))
}

pub fn sync() -> miette::Result<()> {
    let Err(why) = crate::tasks::block_single(twon_persistence::ops::sync::sync()) else {
        return Ok(());
    };

    Err(crate::diagnostics::snapshot_opt_diagnostic(why))
}

pub mod currencies {
    #[derive(clap::Subcommand)]
    pub enum CurrencyCommand {
        #[command(alias = "ls")]
        List,
        #[command(alias = "c")]
        Create {
            #[arg(short, long)]
            symbol: String,
            #[arg(short, long)]
            name: String,
            #[arg(short, long)]
            code: String,
        },
    }

    pub fn create(name: String, symbol: String, code: String) -> miette::Result<()> {
        use twon_persistence::actions::create_currency;

        let result = crate::tasks::block_single({
            let code = code.clone();
            async move {
                let con = match twon_persistence::database::connect().await {
                    Ok(con) => con,
                    Err(why) => twon_persistence::log::database(why),
                };

                twon_persistence::actions::create_currency::run(&con, name, symbol, code).await
            }
        });

        let currency_id = match result {
            Ok(currency_id) => currency_id,
            Err(why) => match why {
                create_currency::Error::Database(err) => twon_persistence::log::database(err),
                create_currency::Error::AlreadyExists => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "currency::AlreadyExists",
                        "Currency with code `{}` already exists",
                        code
                    );

                    return Err(diagnostic.into());
                }
            },
        };

        println!("Currency `{}` created", currency_id);
        Ok(())
    }

    pub fn list() -> miette::Result<()> {
        let result = crate::tasks::block_multi(async move {
            let con = match twon_persistence::database::connect().await {
                Ok(con) => con,
                Err(why) => twon_persistence::log::database(why),
            };

            twon_persistence::actions::list_currencies::run(&con).await
        });

        let currencies = match result {
            Ok(currencies) => currencies,
            Err(err) => twon_persistence::log::database(err),
        };

        for currency in currencies {
            println!(
                "{} `{}` {} {}",
                currency.id, currency.name, currency.symbol, currency.code
            );
        }

        Ok(())
    }
}

pub mod wallets {
    #[derive(clap::Subcommand)]
    pub enum WalletCommand {
        #[command(alias = "ls")]
        List,
    }

    pub fn list() -> miette::Result<()> {
        let wallets = crate::tasks::block_multi(async move {
            let con = match twon_persistence::database::connect().await {
                Ok(con) => con,
                Err(why) => twon_persistence::log::database(why),
            };

            twon_persistence::actions::list_wallets::run(&con).await
        })
        .map_err(crate::diagnostics::snapshot_r_diagnostic)?;

        for wallet in wallets.iter() {
            match &wallet.name {
                Some(name) => println!("{}({}):", name, wallet.id),
                None => println!("`{}`:", wallet.id),
            }
            println!("{:?} {}\n", wallet.currency, wallet.balance);
        }

        Ok(())
    }
}

pub mod actions {
    #[derive(clap::Subcommand)]
    pub enum ActionCommand {
        CreateWallet {
            #[arg(short, long)]
            currency_id: twon_core::CurrencyId,
            #[arg(short, long)]
            name: Option<String>,
        },
    }

    async fn do_create_wallet(
        currency_id: twon_core::CurrencyId,
        name: Option<String>,
    ) -> miette::Result<twon_core::WalletId> {
        let db = twon_persistence::database::connect().await.unwrap();
        let result = twon_persistence::actions::create_wallet::run(&db, currency_id, name).await;

        match result {
            Ok(wallet_id) => Ok(wallet_id),
            Err(e) => Err(crate::diagnostics::snapshot_opt_diagnostic(e)),
        }
    }

    pub fn create_wallet(
        currency_id: twon_core::CurrencyId,
        name: Option<String>,
    ) -> miette::Result<()> {
        let wallet_id = crate::tasks::block_single(do_create_wallet(currency_id, name))?;

        println!("Wallet `{}` created", wallet_id);
        Ok(())
    }
}

pub mod event {
    use twon_core::{Amount, WalletId};

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
            currency_id: twon_core::CurrencyId,
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
                currency: currency_id,
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
