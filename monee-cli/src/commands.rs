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
                print!("{debt_type}Debt {} forgotten", debt_id);
            }
            monee_core::DebtEvent::Accumulate { debt_id, amount } => {
                print!("{debt_type}Debt {} accumulated {}", debt_id, amount);
            }
            monee_core::DebtEvent::Amortize { debt_id, amount } => {
                print!("{debt_type}Debt {} amortized {}", debt_id, amount);
            }
        };
    }

    pub fn handle() -> miette::Result<()> {
        let result = crate::tasks::block_single(async {
            let db = crate::tasks::use_db().await;
            monee::actions::events::list(&db).await
        });

        let events = match result {
            Ok(events) => events,
            Err(why) => monee::log::database(why),
        };

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
                monee_core::Event::InDebt(event) => print_debt_event("In", event),
                monee_core::Event::OutDebt(event) => print_debt_event("Out", event),
            }

            println!(", created at {}", created_at);
        }

        Ok(())
    }
}

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
            print!("{} - ", debt_id);

            let mut actors = actors.iter().peekable();
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
                Some(monee::Entity(_, currency)) => print!("{} {}", currency.code, currency.symbol),
                None => print!("(Unknown currency)"),
            }
            println!("{}", debt.balance);
        }
    }

    pub fn handle(command: DebtsCommand) -> miette::Result<()> {
        match command {
            DebtsCommand::List { show } => match show {
                ShowMode::In => {
                    let debts = get_debts!(|db, snapshot: SnapshotEntry| {
                        monee::actions::debts::list::run_in(db, snapshot.snapshot.in_debts)
                    })?;

                    println!("In debts:");
                    list_debts(&debts);

                    Ok(())
                }
                ShowMode::Out => {
                    let debts = get_debts!(|db, snapshot: SnapshotEntry| {
                        monee::actions::debts::list::run_out(db, snapshot.snapshot.out_debts)
                    })?;

                    println!("Out debts:");
                    list_debts(&debts);

                    Ok(())
                }
                ShowMode::Both => {
                    let result = get_debts!(|db, snapshot: SnapshotEntry| async move {
                        let debts = tokio::try_join!(
                            monee::actions::debts::list::run_in(db, snapshot.snapshot.in_debts,),
                            monee::actions::debts::list::run_out(db, snapshot.snapshot.out_debts,)
                        );

                        Ok(debts)
                    })?;

                    let (in_debts, out_debts) = match result {
                        Ok((in_debts, out_debts)) => (in_debts, out_debts),
                        Err(why) => monee::log::database(why),
                    };

                    println!("In debts:");
                    list_debts(&in_debts);

                    println!("\nOut debts:");
                    list_debts(&out_debts);

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
        let result = crate::tasks::block_single(async move {
            let db = crate::tasks::use_db().await;
            monee::actions::actors::list::run(&db).await
        });

        let actors = match result {
            Ok(actors) => actors,
            Err(why) => monee::log::database(why),
        };

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
        let result = crate::tasks::block_single(async {
            let db = crate::tasks::use_db().await;
            monee::actions::actors::create::run(
                &db,
                monee_core::actor::Actor {
                    name,
                    actor_type,
                    alias: alias.clone(),
                },
            )
            .await
        });

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
    }
}

pub mod do_command {
    #[derive(clap::Args)]
    pub struct DoCommand {
        #[command(subcommand)]
        pub command: DoDetailCommand,

        #[arg(short, long)]
        pub description: Option<String>,
    }

    #[derive(clap::Subcommand)]
    pub enum DoDetailCommand {
        RegisterBalance {
            #[arg(short, long)]
            wallet_id: monee_core::WalletId,
            #[arg(short, long)]
            amount: monee_core::Amount,
        },
        RegisterInDebt {
            #[arg(long)]
            amount: monee_core::Amount,
            #[arg(short, long)]
            currency: crate::args::CurrencyIdOrCode,
            #[arg(long)]
            actor: monee_core::actor::ActorId,
            #[arg(short, long)]
            payment_promise: Option<crate::date::PaymentPromise>,
        },
    }

    pub fn handle(
        DoCommand {
            command,
            description,
        }: DoCommand,
    ) -> miette::Result<()> {
        match command {
            DoDetailCommand::RegisterBalance { wallet_id, amount } => {
                register_balance(wallet_id, amount, description)
            }
            DoDetailCommand::RegisterInDebt {
                amount,
                currency,
                actor,
                payment_promise,
            } => register_in_debt(amount, currency, actor, payment_promise, description),
        }
    }

    fn register_balance(
        wallet_id: monee_core::WalletId,
        amount: monee_core::Amount,
        description: Option<String>,
    ) -> miette::Result<()> {
        use monee::procedures;

        crate::tasks::block_single(async move {
            let con = match monee::database::connect().await {
                Ok(con) => con,
                Err(why) => monee::log::database(why),
            };

            monee::procedures::register_balance(
                &con,
                procedures::CreateProcedure { description },
                procedures::RegisterBalance { wallet_id, amount },
            )
            .await
        })
        .map_err(crate::diagnostics::snapshot_opt_diagnostic)?;

        println!("Done!");

        Ok(())
    }

    fn register_in_debt(
        amount: monee_core::Amount,
        currency: crate::args::CurrencyIdOrCode,
        actor_id: monee_core::actor::ActorId,
        payment_promise: Option<crate::date::PaymentPromise>,
        description: Option<String>,
    ) -> miette::Result<()> {
        use monee::procedures;

        let payment_promise = payment_promise.map(|date| match date {
            crate::date::PaymentPromise::Datetime(datetime) => datetime,
            crate::date::PaymentPromise::Delta(delta) => {
                let mut target = monee::date::Timezone::now();
                delta.add(&mut target);

                target
            }
        });

        let created: miette::Result<bool> = crate::tasks::block_single(async move {
            let db = crate::tasks::use_db().await;
            let Some(currency_id) = crate::args::get_currency(&db, currency, false).await? else {
                return Ok(false);
            };

            monee::procedures::register_in_debt(
                &db,
                procedures::CreateProcedure { description },
                procedures::RegisterInDebt {
                    amount,
                    currency: currency_id,
                    actor_id,
                    payment_promise,
                },
            )
            .await
            .map_err(crate::diagnostics::snapshot_opt_diagnostic)?;

            Ok(true)
        });

        if created? {
            println!("Done!");
        }

        Ok(())
    }
}

use crate::diagnostics::snapshot_read_diagnostic;

pub fn snapshot(output: Option<std::path::PathBuf>) -> miette::Result<()> {
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
    use crate::args::CurrencyIdOrCode;

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
    }

    pub fn deposit(
        wallet_id: monee_core::WalletId,
        amount: monee_core::Amount,
    ) -> miette::Result<()> {
        let event = monee_core::Event::Wallet(monee_core::WalletEvent::Deposit { wallet_id, amount });
        add_event(event)
    }

    pub fn deduct(wallet_id: monee_core::WalletId, amount: monee_core::Amount) -> miette::Result<()> {
        let event = monee_core::Event::Wallet(monee_core::WalletEvent::Deduct { wallet_id, amount });
        add_event(event)
    }

    fn add_event(event: monee_core::Event) -> miette::Result<()> {
        let response = crate::tasks::block_single(async {
            let con = crate::tasks::use_db().await;
            monee::actions::events::add(&con, event).await
        });

        if let Err(why) = response {
            let report = crate::diagnostics::snapshot_opt_diagnostic(why);
            return Err(report);
        }

        Ok(())
    }

    pub fn list() -> miette::Result<()> {
        let wallets = crate::tasks::block_multi(async move {
            let con = match monee::database::connect().await {
                Ok(con) => con,
                Err(why) => monee::log::database(why),
            };

            monee::actions::wallets::list::run(&con).await
        })
        .map_err(crate::diagnostics::snapshot_r_diagnostic)?;

        for wallet in wallets.iter() {
            match &wallet.name {
                Some(name) => print!("{}({}):", name, wallet.id),
                None => print!("`{}`:", wallet.id),
            }

            match &wallet.currency {
                Some(monee::Entity(_, currency)) => print!(" {} {}", currency.code, currency.symbol),
                None => print!("`Unknown currency`"),
            }

            println!("{}\n", wallet.balance);
        }

        Ok(())
    }

    pub fn create(
        currency: CurrencyIdOrCode,
        name: Option<String>,
        yes: bool,
    ) -> miette::Result<()> {
        let result = crate::tasks::block_single(async move {
            let con = crate::tasks::use_db().await;
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