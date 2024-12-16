use money::{MoneyError, MoneyHost};
use serde::Serialize;

use crate::{ActorId, Amount, CurrencyId, DebtId, WalletId};

pub use money::{Money, MoneyMap};

mod money;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    #[serde(flatten)]
    pub money: Money,
}

impl AsMut<Money> for Wallet {
    fn as_mut(&mut self) -> &mut Money {
        &mut self.money
    }
}

impl MoneyHost for Wallet {
    type Key = WalletId;
    type Data = ();

    fn create(money: Money, _: Self::Data) -> Self {
        Self { money }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Debt {
    #[serde(flatten)]
    pub money: Money,
    pub actor_id: ActorId,
}

impl AsMut<Money> for Debt {
    fn as_mut(&mut self) -> &mut Money {
        &mut self.money
    }
}

impl MoneyHost for Debt {
    type Key = DebtId;
    type Data = ActorId;

    fn create(money: Money, data: Self::Data) -> Self {
        Self {
            money,
            actor_id: data,
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub wallets: MoneyMap<Wallet>,
    /// Debts that I should pay
    pub debts: MoneyMap<Debt>,
    /// Debts that should be payed to me
    pub loans: MoneyMap<Debt>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WalletOperation {
    Create {
        wallet_id: WalletId,
        currency_id: CurrencyId,
    },
    /// Does not handle transference before deletion
    Delete {
        wallet_id: WalletId,
    },
    Deposit {
        wallet_id: WalletId,
        amount: Amount,
    },
    Deduct {
        wallet_id: WalletId,
        amount: Amount,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DebtOperation {
    Incur {
        debt_id: DebtId,
        currency_id: CurrencyId,
        actor_id: ActorId,
    },
    /// Does not handle transference before deletion
    Forget {
        debt_id: DebtId,
    },
    Accumulate {
        debt_id: DebtId,
        amount: Amount,
    },
    Amortize {
        debt_id: DebtId,
        amount: Amount,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "group", rename_all = "snake_case")]
pub enum Operation {
    Wallet(WalletOperation),
    Loan(DebtOperation),
    Debt(DebtOperation),
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "monee_error", rename_all = "snake_case")]
pub enum Error {
    Wallet(MoneyError),
    Debt(MoneyError),
    Loan(MoneyError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn print_debt_err(e: &MoneyError, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match e {
                MoneyError::NotFound => write!(f, "debt not found"),
                MoneyError::CannotSub => write!(f, "debt amortize overflow"),
                MoneyError::AlreadyExists => write!(f, "debt already exists"),
            }
        }

        match self {
            Error::Wallet(e) => match e {
                MoneyError::NotFound => write!(f, "wallet not found"),
                MoneyError::CannotSub => write!(f, "cannot deduct from wallet"),
                MoneyError::AlreadyExists => write!(f, "wallet already exists"),
            },
            Error::Debt(e) => {
                write!(f, "in ")?;
                print_debt_err(e, f)
            }
            Error::Loan(e) => {
                write!(f, "out ")?;
                print_debt_err(e, f)
            }
        }
    }
}

impl std::error::Error for Error {}

impl Snapshot {
    pub fn apply(&mut self, event: Operation) -> Result<(), Error> {
        fn apply_debt_operation(
            moneys: &mut MoneyMap<Debt>,
            event: DebtOperation,
        ) -> Result<(), MoneyError> {
            match event {
                DebtOperation::Incur {
                    debt_id,
                    currency_id,
                    actor_id,
                } => moneys.create(debt_id, currency_id, actor_id),
                DebtOperation::Forget { debt_id } => moneys.remove(debt_id),
                DebtOperation::Accumulate { debt_id, amount } => moneys.add(debt_id, amount),
                DebtOperation::Amortize { debt_id, amount } => moneys.sub(debt_id, amount),
            }
        }

        match event {
            Operation::Wallet(operation) => {
                let result = match operation {
                    WalletOperation::Create {
                        wallet_id,
                        currency_id,
                    } => self.wallets.create(wallet_id, currency_id, ()),
                    WalletOperation::Deposit { wallet_id, amount } => {
                        self.wallets.add(wallet_id, amount)
                    }
                    WalletOperation::Deduct { wallet_id, amount } => {
                        self.wallets.sub(wallet_id, amount)
                    }
                    WalletOperation::Delete { wallet_id } => self.wallets.remove(wallet_id),
                };

                result.map_err(Error::Wallet)
            }
            Operation::Debt(operation) => {
                apply_debt_operation(&mut self.debts, operation).map_err(Error::Debt)
            }
            Operation::Loan(operation) => {
                apply_debt_operation(&mut self.loans, operation).map_err(Error::Loan)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_creates_debt() {
        let mut snapshot = Snapshot::default();
        let debt_id = DebtId::new();
        let currency_id = CurrencyId::new();

        let action = DebtOperation::Incur {
            debt_id,
            currency_id,
            actor_id: ActorId::new(),
        };

        snapshot.apply(Operation::Debt(action)).unwrap();

        assert_eq!(snapshot.debts.len(), 1);
    }
}
