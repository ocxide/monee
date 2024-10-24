pub mod application {
    pub mod add {
        use cream::context::ContextProvide;
        use monee_core::{DebtId, WalletId};

        use crate::{
            backoffice::{
                events::domain::{
                    event::{Buy, DebtRegister, Event, MoveValue, PaymentReceived, RegisterBalance},
                    repository::Repository,
                },
                snapshot::application::snapshot_io::SnapshotIO,
            },
            shared::{domain::context::AppContext, infrastructure::errors::AppError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
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
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::ContextProvide;
        use surrealdb::sql::Id;

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
                match &event {
                    Event::Buy(buy) => {
                        let actors = buy
                            .actors
                            .iter()
                            .map(|actor_id| surrealdb::sql::Thing {
                                tb: "actor".into(),
                                id: Id::String(actor_id.to_string()),
                            })
                            .collect::<Vec<_>>();

                        self.0
                            .query("CREATE event 
SET type='buy', item=type::thing('item_tag', $item), amount=$amount, wallet_id=type::thing('wallet', $wallet_id), actors=$actors")
                                .bind(buy).bind(("actors", actors))
                    }
                    Event::RegisterBalance(register) => {
                        self.0
                            .query("CREATE event SET type='register_balance', wallet_id=type::thing('wallet', $wallet_id), amount=$amount")
                            .bind(register)
                    }
                    Event::RegisterDebt(debt) => {
                        self.0
                            .query("CREATE event SET type='register_debt', amount=$amount, currency_id=type::thing('currency', $currency_id), actor_id=type::thing('actor', $actor_id)")
                            .bind(debt)
                    }
                    Event::RegisterLoan(loan) => {
                        self.0
                            .query("CREATE event SET type='register_loan', amount=$amount, currency_id=type::thing('currency', $currency_id), actor_id=type::thing('actor', $actor_id)")
                            .bind(loan)
                    }
                    Event::MoveValue(move_value) => {
                        self.0
                            .query("CREATE event SET type='move_value', from=type::thing('wallet', $from), to=type::thing('wallet', $to), amount=$amount")
                            .bind(move_value)
                    }
                    Event::PaymentReceived(payment) => {
                        self.0
                            .query("CREATE event SET type='payment_received', actor_id=type::thing('actor', $actor_id), wallet_id=type::thing('wallet', $wallet_id), amount=$amount")
                            .bind(payment)
                    }
                }.await?.check()?;

                Ok(())
            }
        }
    }
}
