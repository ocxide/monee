pub mod event {
    use monee_core::{ActorId, Amount, CurrencyId, ItemTagId, WalletId};

    use crate::shared::date::Datetime;

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

    pub fn apply_event(snapshot: &mut monee_core::Snapshot, event: &Event) -> Result<(), Error> {
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

pub mod event_added {
    use cream_events_core::DomainEvent;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct EventAdded {
        pub id: monee_core::EventId,
    }

    impl DomainEvent for EventAdded {
        fn name(&self) -> &'static str {
            "backoffice.events.added"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
