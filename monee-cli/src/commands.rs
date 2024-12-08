pub mod wallet {
    use cream::context::Context;
    use monee::{
        backoffice::wallets::domain::wallet_name::WalletName, prelude::AppContext,
        shared::domain::errors::UniqueSaveError,
    };
    use monee_core::CurrencyId;

    use crate::{alias::MaybeAlias, prelude::MapAppErr};

    #[derive(clap::Subcommand)]
    pub enum WalletCommand {
        Create {
            #[arg(short, long)]
            currency: MaybeAlias<CurrencyId>,

            #[arg(short, long)]
            name: WalletName,

            #[arg(short, long, default_value = "")]
            description: String,
        },
    }

    pub async fn run(ctx: &AppContext, command: WalletCommand) -> miette::Result<()> {
        match command {
            WalletCommand::Create {
                currency,
                name,
                description,
            } => {
                let service =
                    ctx.provide::<monee::backoffice::wallets::application::create_one::CreateOne>();

                let currency_id = currency.resolve(ctx).await?;
                let wallet = monee::backoffice::wallets::domain::wallet::Wallet {
                    description,
                    name,
                    currency_id,
                };

                service.run(wallet).await.map_app_err(ctx, |e| match e {
                    UniqueSaveError::AlreadyExists => miette::diagnostic! {
                        "Wallet with this name already exists"
                    }
                    .into(),
                })
            }
        }
    }
}

pub mod events {
    use cream::context::Context;
    use std::future::Future;

    use crate::prelude::MapAppErr;
    use monee::{
        backoffice::events::domain::event::{Buy, Event, RegisterBalance},
        prelude::AppContext,
    };
    use monee_core::{ActorId, Amount, ItemTagId, WalletId};
    use tokio::{task::JoinSet, try_join};

    use crate::alias::MaybeAlias;

    #[derive(clap::Subcommand)]
    pub enum EventCommand {
        Add {
            #[command(subcommand)]
            command: AddEventCommand,
        },
    }

    #[derive(clap::Subcommand)]
    pub enum AddEventCommand {
        RegisterBalance {
            #[arg(short, long)]
            wallet: MaybeAlias<WalletId>,
            #[arg(short, long)]
            amount: Amount,
        },

        Buy {
            #[arg(short, long)]
            item: MaybeAlias<ItemTagId>,

            #[arg(short, long)]
            actors: Vec<MaybeAlias<ActorId>>,

            #[arg(short, long)]
            wallet: MaybeAlias<WalletId>,

            #[arg(short, long)]
            amount: Amount,
        },
    }

    async fn try_join_collect<T: 'static + Send, E: 'static + Send>(
        futs: impl Iterator<Item = impl Future<Output = Result<T, E>> + 'static + Send>,
    ) -> Result<Vec<T>, E> {
        let mut set: JoinSet<_> = futs.collect();

        let mut res = Vec::new();
        while let Some(next) = set.join_next().await {
            res.push(next.expect("to join")?);
        }

        Ok(res)
    }

    pub async fn run(ctx: &AppContext, command: AddEventCommand) -> Result<(), miette::Error> {
        let service = ctx.provide::<monee::backoffice::events::application::add::Add>();

        let event = match command {
            AddEventCommand::RegisterBalance { wallet, amount } => {
                let wallet_id = wallet.resolve(ctx).await?;
                Event::RegisterBalance(RegisterBalance { amount, wallet_id })
            }

            AddEventCommand::Buy {
                item,
                actors,
                wallet,
                amount,
            } => {
                let wallet_id = wallet.resolve(ctx);
                let item_id = item.resolve(ctx);
                let actors = try_join_collect(actors.into_iter().map(|actor| {
                    let ctx = ctx.clone();
                    async move { actor.resolve(&ctx).await }
                }));

                let (wallet_id, item_id, actors) = try_join!(wallet_id, item_id, actors)?;

                Event::Buy(Buy {
                    actors: actors.into(),
                    amount,
                    item: item_id,
                    wallet_id,
                })
            }
        };

        service.run(event).await.map_app_err(ctx, |err| match err {
            monee::backoffice::events::application::add::Error::Apply(e) => miette::diagnostic! {
                "Failed to apply event {}", e
            }
            .into(),

            monee::backoffice::events::application::add::Error::MoveValue(e) => {
                miette::diagnostic! {
                    "Failed to move value {}",
                    match e {
                        monee::backoffice::events::application::add::MoveValueError::WalletNotFound(_) => "wallet not found",
                        monee::backoffice::events::application::add::MoveValueError::CurrenciesNonEqual => "currencies are not equal",
                    }
                }.into()
            }
        })
    }
}

pub mod currency {
    use cream::context::Context;
    use monee::{
        backoffice::currencies::domain::{
            currency::Currency, currency_code::CurrencyCode, currency_name::CurrencyName,
            currency_symbol::CurrencySymbol,
        },
        prelude::AppContext,
    };

    use crate::{error::LogAndErr, formatted, prelude::MapAppErr};

    #[derive(clap::Subcommand)]
    pub enum CurrencyCommand {
        Create {
            #[arg(short, long)]
            name: CurrencyName,

            #[arg(short, long)]
            code: CurrencyCode,

            #[arg(short, long)]
            symbol: CurrencySymbol,
        },

        #[command(alias = "ls")]
        List,
    }

    pub async fn run(ctx: &AppContext, command: CurrencyCommand) -> miette::Result<()> {
        match command {
            CurrencyCommand::Create { name, code, symbol } => {
                let service =
                    ctx.provide::<monee::backoffice::currencies::application::save_one::SaveOne>();

                let currency = Currency { code, name, symbol };
                service.run(currency).await.map_app_err(ctx, |_| {
                    miette::diagnostic! {
                        "Duplicated currency code",
                    }
                    .into()
                })
            }

            CurrencyCommand::List => {
                let service =
                    ctx.provide::<monee::backoffice::currencies::application::get_all::GetAll>();
                let currencies = service.run().await.log_err(ctx)?;

                crate::output::print_data(currencies.iter().map(|(_, currency)| {
                    formatted!(
                        "{} \"{}\" {}",
                        currency.code,
                        currency.name,
                        currency.symbol
                    )
                }));

                Ok(())
            }
        }
    }
}

pub mod actor {
    use cream::context::Context;
    use monee::{
        backoffice::actors::domain::{
            actor_alias::ActorAlias, actor_name::ActorName, actor_type::ActorType,
        },
        prelude::AppContext,
    };

    use crate::{error::LogAndErr, output::formatted, prelude::MapAppErr};

    #[derive(clap::Subcommand)]
    pub enum ActorCommand {
        Create {
            #[arg(short, long)]
            name: ActorName,

            #[arg(short, long)]
            r#type: ActorType,

            #[arg(short, long)]
            alias: Option<ActorAlias>,
        },

        #[command(alias = "ls")]
        List,
    }

    pub async fn run(ctx: &AppContext, command: ActorCommand) -> miette::Result<()> {
        match command {
            ActorCommand::Create {
                name,
                r#type,
                alias,
            } => {
                let service =
                    ctx.provide::<monee::backoffice::actors::application::create_one::CreateOne>();

                let actor = monee::backoffice::actors::domain::actor::Actor {
                    name,
                    actor_type: r#type,
                    alias,
                };
                service.run(actor).await.map_app_err(ctx, |_| {
                    miette::diagnostic! {
                        "Duplicated actor alias",
                    }
                    .into()
                })
            }

            ActorCommand::List => {
                let service =
                    ctx.provide::<monee::backoffice::actors::application::get_all::GetAll>();
                let actors = service.run().await.log_err(ctx)?;

                crate::output::print_data(actors.iter().map(|(_, actor)| {
                    let actor_alias = match actor.alias.as_ref() {
                        Some(alias) => format!("\"{}\"", alias),
                        None => "<None>".to_owned(),
                    };
                    let actor_type = match actor.actor_type {
                        ActorType::Natural => "Person",
                        ActorType::Business => "Business",
                        ActorType::FinancialEntity => "Financial entity",
                    };

                    formatted!("{} {} - {}", &actor.name, actor_alias, &actor_type)
                }));

                Ok(())
            }
        }
    }
}

pub mod show {
    use cream::context::Context;
    use std::fmt::Display;

    use monee::{reports::snapshot::domain::snapshot::Money, shared::domain::context::AppContext};

    use crate::{formatted, output::Listter, prelude::LogAndErr};

    #[derive(clap::Args)]
    pub struct Args;

    struct MoneyCli(Money);

    impl Display for MoneyCli {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let MoneyCli(money) = self;
            write!(
                f,
                "{} {}{}",
                money.currency.code, money.currency.symbol, money.amount
            )
        }
    }

    fn print_entity<T, D: Display>(
        entities: impl ExactSizeIterator<Item = (T, Money)>,
        entity_display: impl Fn(T) -> D,
    ) {
        let iter = entities.map(|(entity, money)| {
            let entity = (entity_display)(entity);
            let money_cli = MoneyCli(money);
            formatted!("{} => {}", entity, money_cli)
        });
        crate::output::print_iter(Listter::new(iter).map(|item| formatted!("\t{}", item)))
    }

    pub async fn run(ctx: &AppContext, _: Args) -> miette::Result<()> {
        let service =
            ctx.provide::<monee::reports::snapshot::application::snapshot_report::SnapshotReport>();
        let snapshot = service.run().await.log_err(ctx)?;

        println!("Wallets:");
        print_entity(snapshot.wallets.into_values(), |wallet| wallet.name);

        println!("Debts:");
        print_entity(snapshot.debts.into_values(), |debt| {
            formatted!("Debt with '{}'", debt.actor.name)
        });

        println!("Loans:");
        print_entity(snapshot.loans.into_values(), |debt| {
            formatted!("Loan to '{}'", debt.actor.name)
        });

        Ok(())
    }
}
