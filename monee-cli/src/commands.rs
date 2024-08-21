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
