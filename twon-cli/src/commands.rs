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
        #[command(alias = "c")]
        Create {
            #[arg(short, long)]
            currency: IdOrCode,
            #[arg(short, long)]
            name: Option<String>,
            #[arg(short, long, default_value = "false")]
            yes: bool,
        },

        Deduct {
            #[arg(short, long)]
            wallet_id: twon_core::WalletId,
            #[arg(short, long)]
            amount: twon_core::Amount,
        },

        Deposit {
            #[arg(short, long)]
            wallet_id: twon_core::WalletId,
            #[arg(short, long)]
            amount: twon_core::Amount,
        },
    }

    use id_or_code::IdOrCode;
    mod id_or_code {
        use std::str::FromStr;

        #[derive(Clone)]
        pub enum IdOrCode {
            Id(twon_core::CurrencyId),
            Code(String),
        }

        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error(transparent)]
            InvalidId(<twon_core::CurrencyId as FromStr>::Err),
            #[error("Length must be 3 or 4")]
            InvalidLength,
        }

        impl FromStr for IdOrCode {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s.len() == 4 {
                    let id = twon_core::CurrencyId::from_str(s).map_err(Error::InvalidId)?;
                    return Ok(IdOrCode::Id(id));
                }

                if s.len() == 3 {
                    return Ok(IdOrCode::Code(s.to_owned()));
                }

                Err(Error::InvalidLength)
            }
        }
    }

    pub fn deposit(
        wallet_id: twon_core::WalletId,
        amount: twon_core::Amount,
    ) -> miette::Result<()> {
        let event = twon_core::Event::Wallet(twon_core::WalletEvent::Deposit { wallet_id, amount });
        add_event(event)
    }

    pub fn deduct(
        wallet_id: twon_core::WalletId,
        amount: twon_core::Amount,
    ) -> miette::Result<()> {
        let event = twon_core::Event::Wallet(twon_core::WalletEvent::Deduct { wallet_id, amount });
        add_event(event)
    }

    fn add_event(event: twon_core::Event) -> miette::Result<()> {
        let response = crate::tasks::block_single(async {
            let con = match twon_persistence::database::connect().await {
                Ok(con) => con,
                Err(why) => twon_persistence::log::database(why),
            };

            twon_persistence::database::add_event(&con, event).await
        });

        if let Err(why) = response {
            twon_persistence::log::database(why);
        }

        Ok(())
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
                Some(name) => print!("{}({}):", name, wallet.id),
                None => print!("`{}`:", wallet.id),
            }

            match &wallet.currency {
                Some(currency) => print!("{} {}", currency.code, currency.symbol),
                None => print!("`Unknown currency`"),
            }

            println!(" {}\n", wallet.balance);
        }

        Ok(())
    }

    async fn confirm_continue() -> bool {
        use tokio::io::AsyncBufReadExt;
        let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();

        if let Err(why) = stdin.read_line(&mut line).await {
            panic!("Failed to read line: {}", why);
        }

        let answer = line.trim().to_lowercase();
        answer.is_empty() || answer == "y" || answer == "yes"
    }

    pub fn create(currency: IdOrCode, name: Option<String>, yes: bool) -> miette::Result<()> {
        let result = crate::tasks::block_single(async move {
            let con = match twon_persistence::database::connect().await {
                Ok(con) => con,
                Err(why) => twon_persistence::log::database(why),
            };

            let currency_id = match currency {
                IdOrCode::Id(currency_id) => {
                    let exists =
                        match twon_persistence::actions::check_currency_id::run(&con, currency_id)
                            .await
                        {
                            Ok(exists) => exists,
                            Err(err) => twon_persistence::log::database(err),
                        };

                    if !exists && !yes {
                        use tokio::io::AsyncWriteExt;

                        let buf = format!("Currency `{}` not found, continue? (Y/n) ", currency_id);

                        let mut stdout = tokio::io::stdout();
                        stdout.write_all(buf.as_bytes()).await.expect("To write");
                        stdout.flush().await.expect("To flush");

                        let should_continue = confirm_continue().await;

                        if !should_continue {
                            return None;
                        }
                    }

                    currency_id
                }
                IdOrCode::Code(code) => {
                    use twon_persistence::actions::currency_id_from_code;
                    match twon_persistence::actions::currency_id_from_code::run(&con, code.clone())
                        .await
                    {
                        Ok(id) => id,
                        Err(currency_id_from_code::Error::NotFound) => {
                            let diagnostic = miette::diagnostic!(
                                severity = miette::Severity::Error,
                                code = "currency::NotFound",
                                "Currency with code `{}` not found",
                                code
                            );

                            return Some(Err(diagnostic.into()));
                        }
                        Err(currency_id_from_code::Error::Database(error)) => {
                            twon_persistence::log::database(error)
                        }
                    }
                }
            };

            let result = twon_persistence::actions::create_wallet::run(&con, currency_id, name)
                .await
                .map_err(crate::diagnostics::snapshot_opt_diagnostic);

            Some(result)
        });

        let wallet_id = match result {
            Some(Ok(wallet_id)) => wallet_id,
            Some(Err(why)) => return Err(why),
            None => return Ok(()),
        };

        println!("Wallet `{}` created", wallet_id);
        Ok(())
    }
}
