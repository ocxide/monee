pub mod wallet {
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

            #[arg(short, long)]
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
    use crate::prelude::MapAppErr;
    use monee::{
        backoffice::events::domain::event::{Event, RegisterBalance},
        prelude::AppContext,
    };
    use monee_core::{Amount, WalletId};

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
    }

    pub async fn run(ctx: &AppContext, command: AddEventCommand) -> Result<(), miette::Error> {
        let service = ctx.provide::<monee::backoffice::events::application::add::Add>();

        let event = match command {
            AddEventCommand::RegisterBalance { wallet, amount } => {
                let wallet_id = wallet.resolve(ctx).await?;
                Event::RegisterBalance(RegisterBalance { amount, wallet_id })
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
    use monee::{
        backoffice::currencies::domain::{
            currency::Currency, currency_code::CurrencyCode, currency_name::CurrencyName,
            currency_symbol::CurrencySymbol,
        },
        prelude::AppContext,
    };

    use crate::prelude::MapAppErr;

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
        }
    }
}

pub mod actor {
    use monee::{
        backoffice::actors::domain::{
            actor_alias::ActorAlias, actor_name::ActorName, actor_type::ActorType,
        },
        prelude::AppContext,
    };

    use crate::prelude::MapAppErr;

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
        }
    }
}
