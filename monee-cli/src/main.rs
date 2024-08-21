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
    shared::domain::{context::AppContext, errors::UniqueSaveError},
};
use monee_core::CurrencyId;

mod commands;

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
        command: commands::events::EventCommand,
    },

    Currency {
        #[command(subcommand)]
        command: commands::currency::CurrencyCommand,
    },

    Actor {
        #[command(subcommand)]
        command: commands::actor::ActorCommand,
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
            command: commands::events::EventCommand::Add { command },
        } => commands::events::run(ctx, command).await,

        Command::Currency { command } => commands::currency::run(ctx, command).await,

        Command::Actor { command } => commands::actor::run(ctx, command).await,
    }
}
