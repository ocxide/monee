pub mod events {
    fn print_debt_event(debt_type: &str, event: &monee_core::DebtEvent) {
        match event {
            monee_core::DebtEvent::Incur { currency, debt_id } => {
                print!(
                    "{debt_type}Debt {} incurred with currency {}",
                    debt_id, currency
                );
            }
            monee_core::DebtEvent::Forget { debt_id } => {
                print!("{debt_type} {} forgotten", debt_id);
            }
            monee_core::DebtEvent::Accumulate { debt_id, amount } => {
                print!("{debt_type} {} accumulated {}", debt_id, amount);
            }
            monee_core::DebtEvent::Amortize { debt_id, amount } => {
                print!("{debt_type} {} amortized {}", debt_id, amount);
            }
        };
    }

    pub fn handle() -> miette::Result<()> {
        let result: miette::Result<_> = crate::tasks::block_single(async {
            let db = crate::tasks::use_db().await?;

            match monee::actions::events::list(&db).await {
                Ok(events) => Ok(events),
                Err(why) => monee::log::database(why),
            }
        });

        let events = result?;

        for monee::actions::events::EventRow { event, created_at } in events.iter() {
            match event {
                monee_core::Event::Wallet(event) => match event {
                    monee_core::WalletEvent::Create {
                        wallet_id,
                        currency,
                    } => {
                        print!("Wallet {} created with currency {}", wallet_id, currency);
                    }
                    monee_core::WalletEvent::Deduct { wallet_id, amount } => {
                        print!("Wallet {} deducted {}", wallet_id, amount);
                    }
                    monee_core::WalletEvent::Deposit { wallet_id, amount } => {
                        print!("Wallet {} deposited {}", wallet_id, amount);
                    }
                    monee_core::WalletEvent::Delete { wallet_id } => {
                        print!("Wallet {} deleted", wallet_id);
                    }
                },
                monee_core::Event::Debt(event) => print_debt_event("Deby", event),
                monee_core::Event::Loan(event) => print_debt_event("Loan", event),
            }

            println!(", created at {}", created_at);
        }

        Ok(())
    }
}

pub mod item_tags;

pub mod debts {
    use monee::{actions::debts::list::DebtItem, snapshot_io::SnapshotEntry};

    #[derive(clap::Subcommand)]
    pub enum DebtsCommand {
        #[command(alias = "ls")]
        List {
            #[arg(short, long, value_enum, default_value = "both")]
            show: ShowMode,
        },
    }

    #[derive(Clone, Debug, clap::ValueEnum)]
    pub enum ShowMode {
        In,
        Out,
        Both,
    }

    macro_rules! get_debts {
        ($run_list:expr) => {{
            let result: Result<_, miette::Error> = crate::tasks::block_multi(async {
                let db = crate::tasks::use_db();
                let snapshot = monee::snapshot_io::read();
                let (db, snapshot) = tokio::join!(db, snapshot);
                let snapshot_entry =
                    snapshot.map_err(crate::diagnostics::snapshot_read_diagnostic)?;

                let db = db?;
                let result = ($run_list)(&db, snapshot_entry).await;

                match result {
                    Ok(debts) => Ok(debts),
                    Err(why) => monee::log::database(why),
                }
            });

            result
        }};
    }

    fn list_debts(debts: &[DebtItem]) {
        for monee::actions::debts::list::DebtItem {
            debt_id,
            debt,
            actors,
            currency,
        } in debts
        {
            print_debt(
                *debt_id,
                &debt.balance,
                actors.iter(),
                match currency {
                    Some(currency) => Some(&currency.1),
                    None => None,
                },
            );
        }
    }

    pub(in crate::commands) fn print_debt<'a>(
        id: monee_core::DebtId,
        balance: &monee_core::Amount,
        actors: impl Iterator<Item = &'a monee_core::actor::Actor>,
        currency: Option<&monee_core::currency::Currency>,
    ) {
        print!("{} - ", id);

        let mut actors = actors.peekable();
        while let Some(actor) = actors.next() {
            print!("{}", actor.name);
            if let Some(alias) = actor.alias.as_deref() {
                print!("({})", alias);
            }

            if actors.peek().is_some() {
                print!(", ");
            }
        }

        print!(" - ");
        match currency {
            Some(currency) => print!("{} {}", currency.code, currency.symbol),
            None => print!("(Unknown currency)"),
        }
        println!("{}", balance);
    }

    pub fn handle(command: DebtsCommand) -> miette::Result<()> {
        match command {
            DebtsCommand::List { show } => match show {
                ShowMode::In => {
                    let debts = get_debts!(|db, snapshot: SnapshotEntry| {
                        monee::actions::debts::list::run_debts(db, snapshot.snapshot.debts)
                    })?;

                    println!("debts:");
                    list_debts(&debts);

                    Ok(())
                }
                ShowMode::Out => {
                    let debts = get_debts!(|db, snapshot: SnapshotEntry| {
                        monee::actions::debts::list::run_loans(db, snapshot.snapshot.loans)
                    })?;

                    println!("loans");
                    list_debts(&debts);

                    Ok(())
                }
                ShowMode::Both => {
                    let result = get_debts!(|db, snapshot: SnapshotEntry| async move {
                        let debts = tokio::try_join!(
                            monee::actions::debts::list::run_debts(db, snapshot.snapshot.debts),
                            monee::actions::debts::list::run_loans(db, snapshot.snapshot.loans)
                        );

                        Ok(debts)
                    })?;

                    let (debts, loans) = match result {
                        Ok((debts, loans)) => (debts, loans),
                        Err(why) => monee::log::database(why),
                    };

                    println!("debts:");
                    list_debts(&debts);

                    println!("\nloans:");
                    list_debts(&loans);

                    Ok(())
                }
            },
        }
    }
}

pub mod actors {
    #[derive(clap::Subcommand)]
    pub enum ActorsCommand {
        #[command(alias = "ls")]
        List,
        #[command(alias = "c")]
        Create {
            #[arg(short, long)]
            name: String,
            #[arg(short = 't', long = "type")]
            actor_type: monee_core::actor::ActorType,
            #[arg(short, long)]
            alias: Option<String>,
        },
    }

    pub fn handle(command: ActorsCommand) -> miette::Result<()> {
        match command {
            ActorsCommand::List => list(),
            ActorsCommand::Create {
                name,
                actor_type,
                alias,
            } => create(name, actor_type, alias),
        }
    }

    fn list() -> miette::Result<()> {
        let result: miette::Result<_> = crate::tasks::block_single(async move {
            let db = crate::tasks::use_db().await?;

            match monee::actions::actors::list::run(&db).await {
                Ok(actors) => Ok(actors),
                Err(why) => monee::log::database(why),
            }
        });

        let actors = result?;

        for monee::Entity(id, actor) in actors.iter() {
            println!(
                "{} - `{}` {} {}",
                match actor.actor_type {
                    monee_core::actor::ActorType::Natural => "Natural",
                    monee_core::actor::ActorType::Business => "Business",
                    monee_core::actor::ActorType::FinancialEntity => "Financial Entity",
                },
                id,
                actor.name,
                match actor.alias {
                    Some(ref alias) => alias,
                    None => "(no alias)",
                }
            );
        }

        Ok(())
    }

    fn create(
        name: String,
        actor_type: monee_core::actor::ActorType,
        alias: Option<String>,
    ) -> miette::Result<()> {
        crate::tasks::block_single(async {
            let db = crate::tasks::use_db().await?;

            let result = monee::actions::actors::create::run(
                &db,
                monee_core::actor::Actor {
                    name,
                    actor_type,
                    alias: alias.clone(),
                },
            )
            .await;

            let err = match result {
                Ok(id) => {
                    println!("Actor `{}` created", id);
                    return Ok(());
                }
                Err(why) => why,
            };

            match err {
                monee::actions::actors::create::Error::AlreadyExists => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "actor::AlreadyExists",
                        "Actor with alias `{}` already exists",
                        alias.as_deref().unwrap_or_default()
                    );

                    Err(diagnostic.into())
                }
                monee::actions::actors::create::Error::Database(err) => monee::log::database(err),
            }
        })
    }
}

pub mod do_command;

pub mod snapshot {
    pub fn show() -> miette::Result<()> {
        let result: Result<_, miette::Error> = crate::tasks::block_multi(async {
            let db = crate::tasks::use_db().await?;
            let snapshot = monee::actions::snapshopts::show::run(&db)
                .await
                .map_err(crate::diagnostics::snapshot_r_diagnostic)?;

            Ok(snapshot)
        });

        let snapshot = result?;

        println!("Wallets");
        if snapshot.wallets.is_empty() {
            println!("<none>");
        }

        for (id, wallet) in snapshot.wallets {
            crate::commands::wallets::print_wallet(
                id,
                wallet.metadata.name.as_deref(),
                wallet.currency.as_deref(),
                &wallet.money.balance,
            );
        }

        println!("\nDebts");
        if snapshot.debts.is_empty() {
            println!("<none>");
        }

        for (id, debt) in snapshot.debts {
            crate::commands::debts::print_debt(
                id,
                &debt.money.balance,
                debt.actor.iter().map(|a| a.as_ref()),
                debt.currency.as_deref(),
            );
        }

        println!("\nLoans");
        if snapshot.loans.is_empty() {
            println!("<none>");
        }
        for (id, debt) in snapshot.loans {
            crate::commands::debts::print_debt(
                id,
                &debt.money.balance,
                debt.actor.iter().map(|a| a.as_ref()),
                debt.currency.as_deref(),
            );
        }

        Ok(())
    }
}

use crate::diagnostics::snapshot_read_diagnostic;

pub fn snapshot_write(output: Option<std::path::PathBuf>) -> miette::Result<()> {
    let snapshot_entry = monee::snapshot_io::do_read().map_err(snapshot_read_diagnostic)?;

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
    use std::fmt::Write;

    let Err(why) = crate::tasks::block_single(monee::ops::rebuild::rebuild()) else {
        return Ok(());
    };

    let stack = match why {
        monee::ops::rebuild::Error::Apply(error) => error,
        monee::ops::rebuild::Error::Database(e) => monee::log::database(e),
        monee::ops::rebuild::Error::Write(e) => monee::log::snapshot_write(e),
    };

    fn write_event(buf: &mut String, event: &monee::ops::build::EventRow) {
        writeln!(buf, "{:?}, created at {}", event.event, event.created_at)
            .expect("Failed to write preview");
    }

    let mut preview = String::new();
    for event in stack.previous.iter() {
        write_event(&mut preview, event);
    }
    if !stack.previous.is_empty() {
        writeln!(&mut preview, "...").expect("Failed to write preview");
    }

    let start = preview.len();
    write_event(&mut preview, &stack.at);
    let range = (start, preview.len());

    if !stack.next.is_empty() {
        writeln!(&mut preview, "...").expect("Failed to write preview");
    }
    for event in stack.next.iter() {
        write_event(&mut preview, event);
    }

    #[derive(miette::Diagnostic, thiserror::Error, Debug)]
    #[error("Failed to apply snapshot")]
    struct ApplyDiagnostic {
        #[source_code]
        preview: String,

        monee_error: monee_core::Error,
        #[label = "{monee_error}"]
        label: (usize, usize),
    }

    dbg!(&preview);
    dbg!(&stack.snapshot);
    let diagnostic = ApplyDiagnostic {
        preview,
        monee_error: stack.error,
        label: range,
    };

    Err(diagnostic.into())
}

pub fn sync() -> miette::Result<()> {
    let Err(why) = crate::tasks::block_single(monee::ops::sync::sync()) else {
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
        use monee::actions::currencies;

        let result = crate::tasks::block_single({
            let code = code.clone();
            async move {
                let con = match monee::database::connect().await {
                    Ok(con) => con,
                    Err(why) => monee::log::database(why),
                };

                monee::actions::currencies::create::run(&con, name, symbol, code).await
            }
        });

        let currency_id = match result {
            Ok(currency_id) => currency_id,
            Err(why) => match why {
                currencies::create::Error::Database(err) => monee::log::database(err),
                currencies::create::Error::AlreadyExists => {
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
            let con = match monee::database::connect().await {
                Ok(con) => con,
                Err(why) => monee::log::database(why),
            };

            monee::actions::currencies::list::run(&con).await
        });

        let currencies = match result {
            Ok(currencies) => currencies,
            Err(err) => monee::log::database(err),
        };

        for monee::Entity(id, currency) in currencies {
            println!(
                "{} `{}` {} {}",
                id, currency.name, currency.symbol, currency.code
            );
        }

        Ok(())
    }
}

pub mod wallets {
    use monee::actions::wallets;

    use crate::args::CurrencyIdOrCode;

    type Wallet = crate::args::alias::Arg<monee_core::WalletId>;

    #[derive(clap::Subcommand)]
    pub enum WalletCommand {
        #[command(alias = "ls")]
        List,
        #[command(alias = "c")]
        Create {
            #[arg(short, long)]
            currency: CurrencyIdOrCode,
            #[arg(short, long)]
            name: Option<String>,
            #[arg(short, long, default_value = "false")]
            yes: bool,
        },

        Deduct {
            #[arg(short, long)]
            wallet_id: monee_core::WalletId,
            #[arg(short, long)]
            amount: monee_core::Amount,
        },

        Deposit {
            #[arg(short, long)]
            wallet_id: monee_core::WalletId,
            #[arg(short, long)]
            amount: monee_core::Amount,
        },

        Rename {
            #[arg(short, long)]
            wallet: Wallet,

            #[arg(short, long)]
            new_name: String,
        },
    }

    pub fn handle(command: WalletCommand) -> miette::Result<()> {
        match command {
            WalletCommand::List => list(),
            WalletCommand::Create {
                currency,
                name,
                yes,
            } => create(currency, name, yes),
            WalletCommand::Deduct { wallet_id, amount } => deduct(wallet_id, amount),
            WalletCommand::Deposit { wallet_id, amount } => deposit(wallet_id, amount),
            WalletCommand::Rename { wallet, new_name } => rename(wallet, new_name),
        }
    }

    fn rename(wallet: Wallet, new_name: String) -> miette::Result<()> {
        let result: Result<_, miette::Error> = crate::tasks::block_single(async {
            let con = crate::tasks::use_db().await?;
            let wallet_id = crate::args::alias::get_id(&con, wallet).await?;

            monee::actions::wallets::rename::run(&con, wallet_id, &new_name)
                .await
                .map_err(|e| match e {
                    wallets::rename::Error::Database(err) => monee::log::database(err),
                    wallets::rename::Error::NotFound => {
                        miette::miette!(
                            code = "wallet::NotExists",
                            "Wallet `{}` does not exist",
                            wallet_id
                        )
                    }
                    wallets::rename::Error::AlreadyExists => {
                        miette::miette!(
                            code = "wallet::AlreadyExists",
                            "Wallet name `{}` already exists",
                            new_name
                        )
                    }
                })
        });

        result?;

        println!("Wallet renamed");
        Ok(())
    }

    pub fn deposit(
        wallet_id: monee_core::WalletId,
        amount: monee_core::Amount,
    ) -> miette::Result<()> {
        let event =
            monee_core::Event::Wallet(monee_core::WalletEvent::Deposit { wallet_id, amount });
        add_event(event)
    }

    pub fn deduct(
        wallet_id: monee_core::WalletId,
        amount: monee_core::Amount,
    ) -> miette::Result<()> {
        let event =
            monee_core::Event::Wallet(monee_core::WalletEvent::Deduct { wallet_id, amount });
        add_event(event)
    }

    fn add_event(event: monee_core::Event) -> miette::Result<()> {
        crate::tasks::block_single(async {
            let con = crate::tasks::use_db().await?;
            let response = monee::actions::events::add(&con, event).await;

            if let Err(why) = response {
                let report = crate::diagnostics::snapshot_opt_diagnostic(why);
                return Err(report);
            }

            Ok(())
        })
    }

    pub(in crate::commands) fn print_wallet(
        id: monee_core::WalletId,
        name: Option<&str>,
        currency: Option<&monee_core::currency::Currency>,
        balance: &monee_core::Amount,
    ) {
        match name {
            Some(name) => print!("{}({}):", name, id),
            None => print!("`{}`:", id),
        }

        match currency {
            Some(currency) => {
                print!(" {} {}", currency.code, currency.symbol)
            }
            None => print!("`Unknown currency`"),
        }

        println!("{}", balance);
    }

    pub fn list() -> miette::Result<()> {
        let wallets = crate::tasks::block_multi(async move {
            let con = crate::tasks::use_db().await?;

            monee::actions::wallets::list::run(&con)
                .await
                .map_err(crate::diagnostics::snapshot_r_diagnostic)
        })?;

        for wallet in wallets {
            print_wallet(
                wallet.id,
                wallet.name.as_deref(),
                wallet.currency.map(|c| c.1).as_ref(),
                &wallet.balance,
            );
        }

        Ok(())
    }

    pub fn create(
        currency: CurrencyIdOrCode,
        name: Option<String>,
        yes: bool,
    ) -> miette::Result<()> {
        let result = crate::tasks::block_single(async move {
            let con = match crate::tasks::use_db().await {
                Ok(con) => con,
                Err(why) => return Some(Err(why)),
            };

            let currency_id = match crate::args::get_currency(&con, currency, yes).await {
                Ok(Some(currency_id)) => currency_id,
                Ok(None) => return None,
                Err(why) => return Some(Err(why)),
            };

            let result = monee::actions::wallets::create::run(&con, currency_id, name)
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
