pub mod repository {
    use cream::context::FromContext;
    use monee_core::{ActorId, Amount, CurrencyId, EventId, ItemTagId, WalletId};

    use crate::{
        backoffice::events::domain::{event::Event, repository::Repository},
        host::sync::domain::sync_data::EventEntry,
        shared::{
            domain::{context::DbContext, date::Datetime},
            infrastructure::{database::EntityKey, errors::InfrastructureError},
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    impl SurrealRepository {
        pub fn new(connection: crate::shared::infrastructure::database::Connection) -> Self {
            Self(connection)
        }
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "snake_case", tag = "type")]
    pub enum SurrealMoneeEvent {
        Buy {
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
                Event::Buy(buy) => SurrealMoneeEvent::Buy {
                    item: EntityKey(buy.item),
                    amount: buy.amount,
                    wallet_id: EntityKey(buy.wallet_id),
                    actors: IntoIterator::into_iter(buy.actors).map(EntityKey).collect(),
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

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn add(&self, event: Event) -> Result<(), InfrastructureError> {
            let _: Option<()> = self
                .0
                .insert(("event", EventId::default().to_string()))
                .content(SurrealMoneeEvent::from(event))
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

            let _: Vec<()> = self.0.insert("event").content(rows).await?;

            Ok(())
        }
    }
}

