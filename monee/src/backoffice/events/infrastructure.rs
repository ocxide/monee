pub mod repository {
    use cream::context::FromContext;
    use monee_core::{ActorId, Amount, CurrencyId, EventId, ItemTagId, WalletId};
    use monee_types::backoffice::events::event::PaymentReceived;

    use crate::{
        backoffice::events::domain::{event::Event, repository::Repository},
        host::sync::domain::node_changes::EventEntry,
        shared::{
            domain::{context::DbContext, date::Datetime},
            infrastructure::{database::EntityKey, errors::InfrastructureError},
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "snake_case", tag = "type")]
    pub enum SurrealMoneeEvent {
        Purchase {
            item: EntityKey<ItemTagId>,
            amount: Amount,
            wallet_id: EntityKey<WalletId>,
            actors: Vec<EntityKey<ActorId>>,
        },

        RegisterBalance {
            wallet_id: EntityKey<WalletId>,
            amount: Amount,
        },

        RegisterDebt {
            amount: Amount,
            currency_id: EntityKey<CurrencyId>,
            actor_id: EntityKey<ActorId>,
        },

        RegisterLoan {
            amount: Amount,
            currency_id: EntityKey<CurrencyId>,
            actor_id: EntityKey<ActorId>,
        },

        MoveValue {
            from: EntityKey<WalletId>,
            to: EntityKey<WalletId>,
            amount: Amount,
        },

        PaymentReceived {
            actor_id: EntityKey<ActorId>,
            wallet_id: EntityKey<WalletId>,
            amount: Amount,
        },
    }

    impl From<Event> for SurrealMoneeEvent {
        fn from(value: Event) -> Self {
            match value {
                Event::Purchase(purchase) => SurrealMoneeEvent::Purchase {
                    item: EntityKey(purchase.item),
                    amount: purchase.amount,
                    wallet_id: EntityKey(purchase.wallet_id),
                    actors: IntoIterator::into_iter(purchase.actors).map(EntityKey).collect(),
                },
                Event::RegisterBalance(register) => SurrealMoneeEvent::RegisterBalance {
                    wallet_id: EntityKey(register.wallet_id),
                    amount: register.amount,
                },
                Event::RegisterDebt(debt) => SurrealMoneeEvent::RegisterDebt {
                    amount: debt.amount,
                    currency_id: EntityKey(debt.currency_id),
                    actor_id: EntityKey(debt.actor_id),
                },
                Event::RegisterLoan(loan) => SurrealMoneeEvent::RegisterLoan {
                    amount: loan.amount,
                    currency_id: EntityKey(loan.currency_id),
                    actor_id: EntityKey(loan.actor_id),
                },
                Event::MoveValue(move_value) => SurrealMoneeEvent::MoveValue {
                    from: EntityKey(move_value.from),
                    to: EntityKey(move_value.to),
                    amount: move_value.amount,
                },
                Event::PaymentReceived(payment) => SurrealMoneeEvent::PaymentReceived {
                    actor_id: EntityKey(payment.actor_id),
                    wallet_id: EntityKey(payment.wallet_id),
                    amount: payment.amount,
                },
            }
        }
    }

    impl From<SurrealMoneeEvent> for Event {
        fn from(value: SurrealMoneeEvent) -> Self {
            match value {
                SurrealMoneeEvent::Purchase {
                    item,
                    amount,
                    wallet_id,
                    actors,
                } => Event::Purchase(monee_types::backoffice::events::event::Purchase {
                    item: item.0,
                    amount,
                    wallet_id: wallet_id.0,
                    actors: actors.into_iter().map(|k| k.0).collect(),
                }),
                SurrealMoneeEvent::RegisterBalance { wallet_id, amount } => Event::RegisterBalance(
                    monee_types::backoffice::events::event::RegisterBalance {
                        wallet_id: wallet_id.0,
                        amount,
                    },
                ),
                SurrealMoneeEvent::RegisterDebt {
                    amount,
                    currency_id,
                    actor_id,
                } => Event::RegisterDebt(monee_types::backoffice::events::event::DebtRegister {
                    amount,
                    currency_id: currency_id.0,
                    actor_id: actor_id.0,
                    payment_promise: None,
                }),
                SurrealMoneeEvent::RegisterLoan {
                    amount,
                    currency_id,
                    actor_id,
                } => Event::RegisterLoan(monee_types::backoffice::events::event::DebtRegister {
                    amount,
                    currency_id: currency_id.0,
                    actor_id: actor_id.0,
                    payment_promise: None,
                }),
                SurrealMoneeEvent::MoveValue { from, to, amount } => {
                    Event::MoveValue(monee_types::backoffice::events::event::MoveValue {
                        from: from.0,
                        to: to.0,
                        amount,
                    })
                }
                SurrealMoneeEvent::PaymentReceived {
                    actor_id,
                    wallet_id,
                    amount,
                } => Event::PaymentReceived(PaymentReceived {
                    actor_id: actor_id.0,
                    wallet_id: wallet_id.0,
                    amount,
                }),
            }
        }
    }

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn add(&self, id: EventId, event: Event) -> Result<(), InfrastructureError> {
            self.0
                .query("CREATE type::thing('event', $id) CONTENT $event")
                .bind(("id", id))
                .bind(("event", SurrealMoneeEvent::from(event)))
                .await?;

            Ok(())
        }

        async fn save_many(&self, events: Vec<EventEntry>) -> Result<(), InfrastructureError> {
            #[derive(serde::Serialize)]
            struct EventRow {
                id: EntityKey<EventId>,
                #[serde(flatten)]
                event: SurrealMoneeEvent,
                created_at: Datetime,
            }

            let rows: Vec<_> = events
                .into_iter()
                .map(|entry| EventRow {
                    id: EntityKey(entry.id),
                    event: SurrealMoneeEvent::from(entry.event),
                    created_at: entry.created_at,
                })
                .collect();

            self.0
                .query("INSERT INTO event $rows")
                .bind(("rows", rows))
                .await?;

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(unused)]

        use crate::prelude::*;
        use crate::shared::infrastructure::database::connect;

        use super::*;

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn saves_many() {
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let repo: SurrealRepository = ctx.provide();
            repo.save_many(vec![EventEntry {
                id: EventId::default(),
                event: Event::Purchase(monee_types::backoffice::events::event::Purchase {
                    item: ItemTagId::default(),
                    amount: Amount::default(),
                    wallet_id: WalletId::default(),
                    actors: vec![].into(),
                }),
                created_at: Datetime::MIN_UTC,
            }])
            .await
            .unwrap();
        }
    }
}
