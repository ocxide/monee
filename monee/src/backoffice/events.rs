pub mod application {
    pub mod add {
        use cream::context::ContextProvide;
        use monee_core::{DebtId, WalletId};

        use crate::{
            backoffice::{
                events::domain::{
                    event::{Buy, DebtRegister, Event, MoveValue, RegisterBalance},
                    repository::Repository,
                },
                snapshot::domain::repository::SnapshotRepository,
            },
            shared::{domain::context::AppContext, infrastructure::errors::AppError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct Add {
            repository: Box<dyn Repository>,
            snapshot_repository: Box<dyn SnapshotRepository>,
        }

        impl Add {
            pub async fn run(&self, event: Event) -> Result<(), AppError<Error>> {
                let mut snapshot = self.snapshot_repository.read_last().await?;
                if let Err(e) = apply_event(&mut snapshot, &event) {
                    return Err(AppError::App(e));
                }

                self.repository.add(event).await?;
                self.snapshot_repository.save(&snapshot).await?;

                Ok(())
            }
        }

        fn apply_event(snapshot: &mut monee_core::Snapshot, event: &Event) -> Result<(), Error> {
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
                        .as_ref()
                        .get(from)
                        .ok_or(MoveValueError::WalletNotFound(*from))?;

                    let to_wallet = snapshot
                        .wallets
                        .as_ref()
                        .get(to)
                        .ok_or(MoveValueError::WalletNotFound(*to))?;

                    if from_wallet.currency != to_wallet.currency {
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
            };

            Ok(())
        }

        pub enum Error {
            MoveValue(MoveValueError),
            Apply(monee_core::Error),
        }

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
                        currency: self.currency,
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

pub mod domain {
    pub mod repository {
        use crate::shared::infrastructure::errors::InfrastructureError;

        use super::event::Event;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn add(&self, event: Event) -> Result<(), InfrastructureError>;
        }
    }

    pub mod event {
        use monee_core::{ActorId, Amount, CurrencyId, ItemTagId, WalletId};

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
        use cream::context::ContextProvide;

        use crate::{
            backoffice::events::domain::{event::Event, repository::Repository},
            shared::{domain::context::DbContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

        impl SurrealRepository {
            pub fn new(connection: crate::shared::infrastructure::database::Connection) -> Self {
                Self(connection)
            }
        }

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn add(&self, event: Event) -> Result<(), InfrastructureError> {
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
