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
        actor: crate::args::actor::Arg,
        #[arg(short, long)]
        payment_promise: Option<crate::date::PaymentPromise>,
    },

    MoveValue {
        #[arg(short, long)]
        from: monee_core::WalletId,

        #[arg(short, long)]
        to: monee_core::WalletId,

        #[arg(short, long)]
        amount: monee_core::Amount,
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

        DoDetailCommand::MoveValue { from, to, amount } => {
            move_value(from, to, amount, description)
        }
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

        procedures::register_balance::run(
            &con,
            procedures::CreateProcedure { description },
            procedures::register_balance::Plan { wallet_id, amount },
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
    actor: crate::args::actor::Arg,
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
        let db = crate::tasks::use_db().await?;

        let (currency_id, actor) = tokio::try_join!(
            crate::args::get_currency(&db, currency, false),
            crate::args::actor::get_id(&db, actor)
        )?;

        let Some(currency_id) = currency_id else {
            return Ok(false);
        };

        procedures::register_in_debt::run(
            &db,
            procedures::CreateProcedure { description },
            procedures::register_in_debt::Plan {
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
    from: monee_core::WalletId,
    to: monee_core::WalletId,
    amount: monee_core::Amount,
    description: Option<String>,
) -> miette::Result<()> {
    use monee::procedures;

    let result: miette::Result<()> =  crate::tasks::block_single(async move {
        let db = crate::tasks::use_db().await?;

        let result = procedures::move_value::run(
            &db,
            procedures::CreateProcedure { description },
            procedures::move_value::Plan { from, to, amount },
        )
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(procedures::move_value::Error::Snapshot(error)) => Err(crate::diagnostics::snapshot_opt_diagnostic(error)),
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
