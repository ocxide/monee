mod alias;
mod date;

mod prelude {
    pub use crate::error::LogAndErr;
    pub use crate::error::MapAppErr;
}

mod error {
    use monee::shared::{
        domain::context::AppContext,
        infrastructure::errors::{AppError, InfrastructureError},
    };

    pub struct PanicError(InfrastructureError);

    impl PanicError {
        pub fn new(err: InfrastructureError) -> Self {
            Self(err)
        }

        pub fn into_final_report(self, ctx: &AppContext) -> miette::Report {
            let service = ctx.provide::<monee::shared::application::logging::LogService>();
            service.error(self.0);

            miette::diagnostic! {
                "Unhandable error, logging error"
            }
            .into()
        }
    }

    pub trait LogAndErr<T> {
        fn log_err(self, ctx: &AppContext) -> Result<T, miette::Error>;
    }

    impl<T> LogAndErr<T> for Result<T, InfrastructureError> {
        fn log_err(self, ctx: &AppContext) -> Result<T, miette::Error> {
            self.map_err(|e| {
                let err = PanicError::new(e);
                err.into_final_report(ctx)
            })
        }
    }

    pub trait MapAppErr<T, E> {
        fn map_app_err(
            self,
            ctx: &AppContext,
            mapper: impl FnOnce(E) -> miette::Error,
        ) -> Result<T, miette::Error>;
    }

    impl<T, E> MapAppErr<T, E> for Result<T, AppError<E>> {
        fn map_app_err(
            self,
            ctx: &AppContext,
            mapper: impl FnOnce(E) -> miette::Error,
        ) -> Result<T, miette::Error> {
            self.map_err(|e| match e {
                AppError::Infrastructure(e) => PanicError::new(e).into_final_report(ctx),
                AppError::App(e) => mapper(e),
            })
        }
    }
}

use alias::MaybeAlias;
use clap::Parser;
use error::MapAppErr;
use monee::{
    backoffice::wallets::domain::wallet_name::WalletName,
    prelude::AppError,
    shared::domain::{context::AppContext, errors::UniqueSaveError},
};
use monee_core::CurrencyId;

mod events_commands {
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

#[derive(clap::Parser)]
struct CliParser {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Wallet {
        #[command(subcommand)]
        command: WalletCommand,
    },

    Events {
        #[command(subcommand)]
        command: events_commands::EventCommand,
    },
}

#[derive(clap::Subcommand)]
enum WalletCommand {
    Create {
        #[arg(short, long)]
        currency: MaybeAlias<CurrencyId>,

        #[arg(short, long)]
        name: WalletName,

        #[arg(short, long)]
        description: String,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let (ctx, main_task) = monee::shared::domain::context::setup()
        .await
        .expect("To setup context");

    let handle = tokio::spawn(main_task);

    let cli = CliParser::parse();
    run(&ctx, cli).await?;

    handle.abort();
    Ok(())
}

async fn run(ctx: &AppContext, cli: CliParser) -> miette::Result<()> {
    match cli.command {
        Command::Wallet { command } => match command {
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
        },

        Command::Events {
            command: events_commands::EventCommand::Add { command },
        } => events_commands::run(ctx, command).await,
    }
}
