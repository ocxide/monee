pub mod application {
    pub mod get_events {
        use cream::context::FromContext;

        use crate::{
            reports::events::domain::{event::Event, repository::Repository},
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct GetEvents {
            repository: Box<dyn Repository>,
        }

        impl GetEvents {
            pub async fn run(&self) -> Result<Vec<Event>, InfrastructureError> {
                self.repository.get_all().await
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use crate::shared::infrastructure::errors::InfrastructureError;

        use super::event::Event;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn get_all(&self) -> Result<Vec<Event>, InfrastructureError>;
        }
    }

    pub mod event {
        use monee_core::Amount;

        use crate::backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
            wallets::domain::wallet_name::WalletName,
        };

        #[derive(serde::Deserialize, Debug)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum Event {
            Buy {
                item: String,
                actors: Box<[Actor]>,
                wallet: WalletName,
                amount: Amount,
            },
            MoveValue {
                from: WalletName,
                to: WalletName,
                amount: Amount,
            },
            RegisterBalance {
                wallet: WalletName,
                amount: Amount,
            },

            RegisterDebt(DebtRegister),
            RegisterLoan(DebtRegister),
        }

        #[derive(serde::Deserialize, Debug)]
        pub struct DebtRegister {
            pub amount: Amount,
            pub currency: Currency,
            pub actor: Actor,
            pub payment_promise: Option<crate::date::Datetime>,
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::FromContext;

        use crate::{
            reports::events::domain::{event::Event, repository::Repository},
            shared::{
                domain::context::DbContext,
                infrastructure::{database::Connection, errors::InfrastructureError},
            },
        };

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn get_all(&self) -> Result<Vec<Event>, InfrastructureError> {
                let mut response = self
                    .0
                    .query(
                        "SELECT type, amount, wallet_id.name as wallet,
item.name as item, actors, 
from.name as from, to.name as to,
currency_id as currency, actor_id as actor, payment_promise FROM event FETCH actors, currency, actor",
                    )
                    .await?
                    .check()?;

                Ok(response.take(0)?)
            }
        }

        #[cfg(all(test, feature = "db_test"))]
        mod tests {
            use std::str::FromStr;
            use monee_core::{ActorId, Amount, CurrencyId, ItemTagId, WalletId};

            use super::*;
            use crate::backoffice::{
                actors::domain::{actor::Actor, actor_type::ActorType, repository::Repository as _}, currencies::domain::currency::Currency, events::domain::{event::{Buy, DebtRegister, Event as AddEvent}, repository::Repository as _}, item_tags::domain::{item_tag::ItemTag, repository::Repository as _}, wallets::domain::{repository::Repository as _, wallet::Wallet},
                currencies::domain::repository::Repository as _
            };

            #[test]
            fn can_get_buy_events() {
                return;
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    let db = crate::shared::infrastructure::database::connect().await.unwrap();
                    let ctx = crate::shared::domain::context::DbContext::new(db);

                    let repo: super::SurrealRepository = ctx.provide();
                    let save_repo: crate::backoffice::events::infrastructure::repository::SurrealRepository = ctx.provide();

                    let actor_repo: crate::backoffice::actors::infrastructure::repository::SurrealRepository = ctx.provide();
                    let item_repo: crate::backoffice::item_tags::infrastructure::repository::SurrealRepository = ctx.provide();
                    let wallet_repo: crate::backoffice::wallets::infrastructure::repository::SurrealRepository = ctx.provide();

                    let actor_id = ActorId::new();
                    let actor = Actor {
                        name: "actor1".into(),
                        actor_type: ActorType::Natural,
                        alias: None,
                    };
                    actor_repo.save(actor_id, actor).await.unwrap();

                    let item_id = ItemTagId::new();
                    item_repo.save(item_id, ItemTag { name: "item_1".into() }).await.unwrap();

                    let wallet_id = WalletId::new();
                    let wallet = Wallet {
                        currency_id: CurrencyId::new(),
                        name: "wallet_1".parse().unwrap(),
                        description: "".into(),
                    };
                    wallet_repo.save(wallet_id, wallet).await.unwrap();

                    save_repo.add(AddEvent::Buy(Buy {
                        item: item_id,
                        actors: vec![actor_id].into(),
                        wallet_id: WalletId::new(),
                        amount: "1.00".parse().unwrap(),
                    })).await.unwrap();

                    let events = repo.get_all().await;
                    println!("{:#?}", events);
                });
            }

            #[test]
            fn can_get_debt_events() {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    let db = crate::shared::infrastructure::database::connect().await.unwrap();
                    let ctx = crate::shared::domain::context::DbContext::new(db);

                    let repo: super::SurrealRepository = ctx.provide();
                    let save_repo: crate::backoffice::events::infrastructure::repository::SurrealRepository = ctx.provide();

                    let actor_repo: crate::backoffice::actors::infrastructure::repository::SurrealRepository = ctx.provide();
                    let currency_repo: crate::backoffice::currencies::infrastructure::repository::SurrealRepository = ctx.provide();

                    let actor_id = ActorId::new();
                    let actor = Actor {
                        name: "actor1".into(),
                        actor_type: ActorType::Natural,
                        alias: None,
                    };
                    actor_repo.save(actor_id, actor).await.unwrap();

                    let currency_id = CurrencyId::new();
                    currency_repo.save(currency_id, Currency {
                        name: "currency_1".into(),
                        symbol: "symbol_1".into(),
                        code: "code_1".into(),
                    }).await.unwrap();

                    save_repo.add(AddEvent::RegisterDebt(DebtRegister {
                        amount: "1.00".parse().unwrap(),
                        currency_id,
                        actor_id,
                        payment_promise: None,
                    })).await.unwrap();

                    let events = repo.get_all().await.unwrap();
                    panic!("{:#?}", events);
                });
            }
        }
    }
}
