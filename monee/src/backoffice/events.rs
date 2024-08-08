pub mod application {
    pub mod add {
        use cream::from_context::FromContext;

        use crate::{
            backoffice::events::domain::{event::Event, repository::Repository},
            shared::domain::context::AppContext,
        };

        pub struct Add {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for Add {
            fn from_context(ctx: &C) -> Self {
                Self {
                    repository: ctx.backoffice_events_repository(),
                }
            }
        }

        impl Add {
            pub async fn run(&self, event: Event) -> Result<(), Error> {

            }
        }

        pub enum Error {}
    }
}

pub mod domain {
    pub mod repository {
        use crate::shared::infrastructure::errors::UnspecifiedError;

        use super::event::Event;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn add(&self, event: Event) -> Result<(), UnspecifiedError>;
        }
    }

    pub mod event {
        use monee_core::{actor::ActorId, item_tag::ItemTagId, Amount, CurrencyId, WalletId};

        use crate::date::Datetime;

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct DebtRegister {
            pub amount: Amount,
            pub currency: CurrencyId,
            pub actor_id: ActorId,
            pub payment_promise: Option<Datetime>,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct Buy {
            pub item: ItemTagId,
            pub actors: Box<[ActorId]>,
            pub wallet_id: WalletId,
            pub amount: Amount,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct MoveValue {
            pub from: WalletId,
            pub to: WalletId,
            pub amount: Amount,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct RegisterBalance {
            pub wallet_id: WalletId,
            pub amount: Amount,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum Event {
            Buy(Buy),
            MoveValue(MoveValue),
            RegisterBalance(RegisterBalance),
            RegisterDebt(DebtRegister),
            RegisterLoan(DebtRegister),
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use crate::{
            backoffice::events::domain::{event::Event, repository::Repository},
            shared::infrastructure::errors::UnspecifiedError,
        };

        pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

        impl SurrealRepository {
            pub fn new(connection: crate::shared::infrastructure::database::Connection) -> Self {
                Self(connection)
            }
        }

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn add(&self, event: Event) -> Result<(), UnspecifiedError> {
                self.0
                    .query("CREATE event CONTENT $event")
                    .bind(("event", event))
                    .await?
                    .check()?;

                Ok(())
            }
        }
    }
}
