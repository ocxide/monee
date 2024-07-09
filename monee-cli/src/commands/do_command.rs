use std::future::Future;

use monee::procedures;

#[derive(clap::Args)]
pub struct DoCommand {
    #[command(subcommand)]
    pub command: DoDetailCommand,

    #[arg(short, long)]
    pub description: Option<String>,
}

pub type Wallet = crate::args::alias::Arg<monee_core::WalletId>;
pub type Actor = crate::args::alias::Arg<monee_core::actor::ActorId>;

#[derive(clap::Subcommand)]
pub enum DoDetailCommand {
    RegisterBalance {
        #[arg(short, long)]
        wallet: Wallet,

        #[arg(short, long)]
        amount: monee_core::Amount,
    },

    RegisterDebt(RegisterDebt),
    RegisterLoan(RegisterDebt),

    MoveValue {
        #[arg(short, long)]
        from: Wallet,

        #[arg(short, long)]
        to: Wallet,

        #[arg(short, long)]
        amount: monee_core::Amount,
    },

    Buy {
        #[arg(short, long)]
        wallet: Wallet,

        #[arg(short, long)]
        amount: monee_core::Amount,

        #[arg(short, long)]
        items: Vec<String>,

        #[arg(short, long)]
        from_actor: Vec<Actor>,
    },
}

#[derive(clap::Args)]
pub struct RegisterDebt {
    #[arg(long)]
    amount: monee_core::Amount,
    #[arg(short, long)]
    currency: crate::args::CurrencyIdOrCode,
    #[arg(long)]
    actor: Actor,
    #[arg(short, long)]
    payment_promise: Option<crate::date::PaymentPromise>,
}

pub fn handle(
    DoCommand {
        command,
        description,
    }: DoCommand,
) -> miette::Result<()> {
    match command {
        DoDetailCommand::RegisterBalance { wallet, amount } => {
            register_balance(wallet, amount, description)
        }

        DoDetailCommand::RegisterDebt(arg) => {
            register_any_debt(description, arg, |db, procedure, plan| async move {
                procedures::register_debt::run_debt(&db, procedure, plan).await
            })
        }

        DoDetailCommand::RegisterLoan(arg) => {
            register_any_debt(description, arg, |db, procedure, plan| async move {
                procedures::register_debt::run_loan(&db, procedure, plan).await
            })
        }

        DoDetailCommand::MoveValue { from, to, amount } => {
            move_value(from, to, amount, description)
        }

        DoDetailCommand::Buy {
            wallet,
            amount,
            items,
            from_actor,
        } => buy(description, wallet, amount, items, from_actor),
    }
}

fn register_balance(
    wallet: Wallet,
    amount: monee_core::Amount,
    description: Option<String>,
) -> miette::Result<()> {
    use monee::procedures;

    let result: miette::Result<_> = crate::tasks::block_single(async move {
        let db = crate::tasks::use_db().await?;
        let wallet_id = crate::args::alias::get_id(&db, wallet).await?;

        procedures::register_balance::run(
            &db,
            procedures::CreateProcedure { description },
            procedures::register_balance::Plan { wallet_id, amount },
        )
        .await
        .map_err(crate::diagnostics::snapshot_opt_diagnostic)
    });

    result?;

    println!("Done!");

    Ok(())
}

fn register_any_debt<F, Fut>(
    description: Option<String>,
    RegisterDebt {
        actor,
        amount,
        currency,
        payment_promise,
    }: RegisterDebt,
    run: F,
) -> miette::Result<()>
where
    F: Fn(
        monee::database::Connection,
        procedures::CreateProcedure,
        procedures::register_debt::Plan,
    ) -> Fut,
    Fut: Future<Output = Result<(), monee::error::SnapshotOptError>>,
{
    let payment_promise = payment_promise.map(|date| match date {
        crate::date::PaymentPromise::Datetime(datetime) => datetime,
        crate::date::PaymentPromise::Delta(delta) => {
            let mut target = monee::date::Timezone::now();
            delta.add(&mut target);

            target
        }
    });

    let created: miette::Result<bool> = crate::tasks::block_single(async move {
        let db = crate::tasks::use_db().await?;

        let (currency_id, actor) = tokio::try_join!(
            crate::args::get_currency(&db, currency, false),
            crate::args::alias::get_id(&db, actor)
        )?;

        let Some(currency_id) = currency_id else {
            return Ok(false);
        };

        (run)(
            db,
            procedures::CreateProcedure { description },
            procedures::register_debt::Plan {
                amount,
                currency: currency_id,
                actor_id: actor,
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

fn move_value(
    from: Wallet,
    to: Wallet,
    amount: monee_core::Amount,
    description: Option<String>,
) -> miette::Result<()> {
    use monee::procedures;

    let result: miette::Result<()> = crate::tasks::block_single(async move {
        let db = crate::tasks::use_db().await?;
        let (from, to) = tokio::try_join!(
            crate::args::alias::get_id(&db, from),
            crate::args::alias::get_id(&db, to)
        )?;

        let result = procedures::move_value::run(
            &db,
            procedures::CreateProcedure { description },
            procedures::move_value::Plan { from, to, amount },
        )
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(procedures::move_value::Error::Snapshot(error)) => {
                Err(crate::diagnostics::snapshot_opt_diagnostic(error))
            }
            Err(procedures::move_value::Error::UnequalCurrencies) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "wallets::UnequalCurrencies",
                    "Cannot move value between wallets with different currencies"
                );

                Err(diagnostic.into())
            }
        }
    });

    result?;

    println!("Done!");
    Ok(())
}

async fn resolve_many<T, F, Fut, M>(
    db: &monee::database::Connection,
    things: Vec<T>,
    f: F,
) -> miette::Result<Vec<M>>
where
    T: Send + Sync + 'static,
    F: Fn(monee::database::Connection, T) -> Fut + Send + Copy + 'static,
    Fut: Future<Output = miette::Result<M>> + Send,
    M: Send + Sync + 'static,
{
    let mut items = vec![];
    let mut set = tokio::task::JoinSet::new();

    for thing in things {
        let db = db.clone();
        set.spawn(async move {
            (f)(db, thing).await
        });
    }

    while let Some(result) = set.join_next().await {
        match result.expect("Failed to join task") {
            Ok(thing) => items.push(thing),
            Err(error) => {
                return Err(error);
            }
        }
    }

    Ok(items)
}

fn buy(
    description: Option<String>,
    wallet: Wallet,
    amount: monee_core::Amount,
    item_names: Vec<String>,
    from_actors: Vec<Actor>,
) -> miette::Result<()> {
    crate::tasks::block_multi(async move {
        let db = crate::tasks::use_db().await?;

        let actors = resolve_many(&db, from_actors, |db, actor| async move {
            crate::args::alias::get_id(&db, actor).await
        });

        let items = resolve_many(&db, item_names, |db, name| async move {
            let result = monee::actions::item_tags::get::run(&db, name.as_str()).await;
            match result {
                Ok(Some(item)) => Ok(item),
                Ok(None) => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "item_tag::NotFound",
                        "Item tag `{}` not found",
                        name
                    );

                    Err(diagnostic.into())
                }
                Err(why) => monee::log::database(why),
            }
        });

        let wallet_id = crate::args::alias::get_id(&db, wallet);

        let (actors, items, wallet_id) = tokio::try_join!(actors, items, wallet_id)?;
        monee::procedures::buy::run(
            &db,
            monee::procedures::CreateProcedure { description },
            monee::procedures::buy::Plan {
                wallet_id,
                amount,
                items,
                from_actors: actors,
            },
        )
        .await
        .map_err(crate::diagnostics::snapshot_opt_diagnostic)
    })
    .inspect(|_| println!("Done!"))
}
