pub mod actors;
pub mod wallets;

pub mod events {
    pub mod application {
        pub mod add_buy {
            use cream::from_context::FromContext;

            use crate::{
                backoffice::events::domain::{event::Buy, repository::Repository},
                shared::{domain::context::AppContext, errors::InfrastructureError},
            };

            pub struct AddBuy {
                repository: Box<dyn Repository>,
            }

            impl<C: AppContext> FromContext<C> for AddBuy {
                fn from_context(context: &C) -> Self {
                    Self {
                        repository: context.backoffice_events_repository(),
                    }
                }
            }

            impl AddBuy {
                pub async fn run(&self, event: Buy) -> Result<(), InfrastructureError> {
                    self.repository.add_buy(event).await
                }
            }
        }
    }

    pub mod domain {
        pub mod repository {
            use crate::shared::errors::InfrastructureError;

            use super::event::Buy;

            #[async_trait::async_trait]
            pub trait Repository {
                async fn add_buy(&self, event: Buy) -> Result<(), InfrastructureError>;
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
            use crate::backoffice::events::domain::{event::Buy, repository::Repository};

            pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

            #[async_trait::async_trait]
            impl Repository for SurrealRepository {
                async fn add_buy(
                    &self,
                    event: Buy,
                ) -> Result<(), crate::shared::errors::InfrastructureError> {
                    todo!()
                }
            }
        }
    }
}
