mod alias;
mod date;

mod error {
    use monee::shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError};

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
}

use alias::MaybeAlias;
use clap::Parser;
use monee::backoffice::wallets::domain::wallet_name::WalletName;
use monee_core::CurrencyId;

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
}

#[derive(clap::Subcommand)]
enum WalletCommand {
    Create {
        currency: MaybeAlias<CurrencyId>,
        name: WalletName,
        description: String,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let (ctx, main_task) = monee::shared::domain::context::setup()
        .await
        .expect("To setup context");

    tokio::spawn(main_task);

    let cli = CliParser::parse();
    match cli.command {
        Command::Wallet { command } => match command {
            WalletCommand::Create {
                currency,
                name,
                description,
            } => {
                let service =
                    ctx.provide::<monee::backoffice::wallets::application::create_one::CreateOne>();

                let currency_id = currency.resolve(&ctx).await?;
                let wallet = monee::backoffice::wallets::domain::wallet::Wallet {
                    description,
                    name,
                    currency_id,
                };

                service
                    .run(wallet)
                    .await
                    .map_err(|e| error::PanicError::new(e).into_final_report(&ctx))
                    .map(|_| ())
            }
        },
    }
}
