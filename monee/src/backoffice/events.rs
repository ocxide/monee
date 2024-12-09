pub mod application {
    pub mod add {
        use cream::context::FromContext;

        use crate::{
            backoffice::{
                events::domain::{
                    apply_event::{apply_event, Error},
                    event::Event,
                    repository::Repository,
                },
                snapshot::application::snapshot_io::SnapshotIO,
            },
            shared::{domain::context::AppContext, infrastructure::errors::AppError},
        };

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct Add {
            repository: Box<dyn Repository>,
            snapshot_io: SnapshotIO,
        }

        impl Add {
            pub async fn run(&self, event: Event) -> Result<(), AppError<Error>> {
                let mut snapshot = self.snapshot_io.read_last().await?;
                if let Err(e) = apply_event(&mut snapshot, &event) {
                    return Err(AppError::App(e));
                }

                self.repository.add(event).await?;
                self.snapshot_io.save(snapshot).await?;

                Ok(())
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_core::EventId;

        use crate::{
            host::sync::domain::sync_data::EventEntry,
            shared::infrastructure::errors::InfrastructureError,
        };

        use super::event::Event;

        #[async_trait::async_trait]
        pub trait Repository: 'static + Send + Sync {
            async fn add(&self, event: Event) -> Result<(), InfrastructureError>;
            async fn save_many(
                &self,
                events: Vec<(EventId, EventEntry)>,
            ) -> Result<(), InfrastructureError>;
        }
    }

    pub mod event {
        use monee_core::{ActorId, Amount, CurrencyId, ItemTagId, WalletId};

        use crate::shared::domain::date::Datetime;

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct DebtRegister {
            pub amount: Amount,
            pub currency_id: CurrencyId,
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
        pub struct PaymentReceived {
            pub actor_id: ActorId,
            pub wallet_id: WalletId,
            pub amount: Amount,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case", tag = "type")]
        pub enum Event {
            Buy(Buy),
            MoveValue(MoveValue),
            RegisterBalance(RegisterBalance),
            RegisterDebt(DebtRegister),
            RegisterLoan(DebtRegister),
            PaymentReceived(PaymentReceived),
        }
    }

    pub mod apply_event {
        use monee_core::{DebtId, WalletId};

        use super::event::{Buy, DebtRegister, Event, MoveValue, PaymentReceived, RegisterBalance};

        pub fn apply_event(
            snapshot: &mut monee_core::Snapshot,
            event: &Event,
        ) -> Result<(), Error> {
            match event {
                Event::Buy(Buy {
                    amount, wallet_id, ..
                }) => snapshot.apply(monee_core::Operation::Wallet(
                    monee_core::WalletOperation::Deduct {
                        wallet_id: *wallet_id,
                        amount: *amount,
                    },
                ))?,
                Event::RegisterBalance(RegisterBalance {
                    amount, wallet_id, ..
                }) => snapshot.apply(monee_core::Operation::Wallet(
                    monee_core::WalletOperation::Deposit {
                        wallet_id: *wallet_id,
                        amount: *amount,
                    },
                ))?,
                Event::RegisterDebt(debt_register) => {
                    for operation in debt_register.create_operators() {
                        snapshot.apply(monee_core::Operation::Debt(operation))?;
                    }
                }
                Event::RegisterLoan(debt_register) => {
                    for operation in debt_register.create_operators() {
                        snapshot.apply(monee_core::Operation::Loan(operation))?;
                    }
                }
                Event::MoveValue(MoveValue { amount, to, from }) => {
                    let from_wallet = snapshot
                        .wallets
                        .get(from)
                        .ok_or(MoveValueError::WalletNotFound(*from))?;

                    let to_wallet = snapshot
                        .wallets
                        .get(to)
                        .ok_or(MoveValueError::WalletNotFound(*to))?;

                    if from_wallet.money.currency_id != to_wallet.money.currency_id {
                        return Err(MoveValueError::CurrenciesNonEqual.into());
                    }

                    snapshot.apply(monee_core::Operation::Wallet(
                        monee_core::WalletOperation::Deduct {
                            wallet_id: *from,
                            amount: *amount,
                        },
                    ))?;

                    snapshot.apply(monee_core::Operation::Wallet(
                        monee_core::WalletOperation::Deposit {
                            wallet_id: *to,
                            amount: *amount,
                        },
                    ))?;
                }
                Event::PaymentReceived(PaymentReceived {
                    wallet_id, amount, ..
                }) => {
                    snapshot.apply(monee_core::Operation::Wallet(
                        monee_core::WalletOperation::Deposit {
                            wallet_id: *wallet_id,
                            amount: *amount,
                        },
                    ))?;
                }
            };

            Ok(())
        }

        #[derive(serde::Serialize)]
        #[serde(rename_all = "snake_case", tag = "type", content = "error")]
        pub enum Error {
            MoveValue(MoveValueError),
            Apply(monee_core::Error),
        }

        #[derive(serde::Serialize)]
        #[serde(rename_all = "snake_case", tag = "move_error")]
        pub enum MoveValueError {
            CurrenciesNonEqual,
            WalletNotFound(WalletId),
        }

        impl From<MoveValueError> for Error {
            fn from(value: MoveValueError) -> Self {
                Self::MoveValue(value)
            }
        }

        impl From<monee_core::Error> for Error {
            fn from(value: monee_core::Error) -> Self {
                Self::Apply(value)
            }
        }

        impl DebtRegister {
            fn create_operators(&self) -> [monee_core::DebtOperation; 2] {
                let debt_id = DebtId::new();
                [
                    monee_core::DebtOperation::Incur {
                        currency_id: self.currency_id,
                        actor_id: self.actor_id,
                        debt_id,
                    },
                    monee_core::DebtOperation::Accumulate {
                        debt_id,
                        amount: self.amount,
                    },
                ]
            }
        }
    }
}

pub mod infrastructure {
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

            async fn save_many(
                &self,
                events: Vec<(EventId, EventEntry)>,
            ) -> Result<(), InfrastructureError> {
                #[derive(serde::Serialize)]
                struct EventRow {
                    id: EntityKey<EventId>,
                    #[serde(flatten)]
                    event: SurrealMoneeEvent,
                    created_at: Datetime,
                }

                let rows: Vec<_> = events
                    .into_iter()
                    .map(|(id, entry)| EventRow {
                        id: EntityKey(id),
                        event: SurrealMoneeEvent::from(entry.event),
                        created_at: entry.created_at,
                    })
                    .collect();

                let _: Vec<()> = self.0.insert("event").content(rows).await?;

                Ok(())
            }
        }
    }
}
