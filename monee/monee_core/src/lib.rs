mod amount;

pub use amount::Amount;
pub use money_record::{MoneyRecord, MoneyStorage};

mod ids;
pub use ids::*;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub wallets: money_record::MoneyRecord<WalletId>,
    /// Debts that I should pay
    pub debts: money_record::MoneyRecord<DebtId>,
    /// Debts that should be payed to me
    pub loans: money_record::MoneyRecord<DebtId>,
}

pub mod money_record {
    use std::collections::HashMap;

    #[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(bound = "K: serde::Serialize + serde::de::DeserializeOwned")]
    pub struct MoneyRecord<K: Eq + std::hash::Hash>(HashMap<K, MoneyStorage>);

    impl<K: Eq + std::hash::Hash> IntoIterator for MoneyRecord<K> {
        type Item = <HashMap<K, MoneyStorage> as IntoIterator>::Item;
        type IntoIter = <HashMap<K, MoneyStorage> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    impl<K: std::hash::Hash + Eq> AsRef<HashMap<K, MoneyStorage>> for MoneyRecord<K> {
        fn as_ref(&self) -> &HashMap<K, MoneyStorage> {
            &self.0
        }
    }

    impl<K: Eq + std::hash::Hash> MoneyRecord<K> {
        pub(crate) fn apply(&mut self, key: K, action: Action) -> Result<(), Error> {
            match action {
                Action::Add(amount) => {
                    if let Some(storage) = self.0.get_mut(&key) {
                        storage.balance += amount;
                        Ok(())
                    } else {
                        Err(Error::NotFound)
                    }
                }

                Action::Sub(amount) => {
                    if let Some(storage) = self.0.get_mut(&key) {
                        let result = storage
                            .balance
                            .checked_sub(amount)
                            .ok_or(Error::CannotSub)?;

                        storage.balance = result;
                        Ok(())
                    } else {
                        Err(Error::NotFound)
                    }
                }

                Action::Create(currency) => {
                    if let std::collections::hash_map::Entry::Vacant(e) = self.0.entry(key) {
                        e.insert(MoneyStorage {
                            balance: crate::Amount::default(),
                            currency,
                        });
                        Ok(())
                    } else {
                        Err(Error::AlreadyExists)
                    }
                }

                Action::Remove => {
                    if self.0.contains_key(&key) {
                        self.0.remove(&key);
                        Ok(())
                    } else {
                        Err(Error::NotFound)
                    }
                }
            }
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct MoneyStorage {
        pub balance: crate::Amount,
        pub currency: crate::CurrencyId,
    }

    #[derive(Debug)]
    pub enum Error {
        NotFound,
        CannotSub,
        AlreadyExists,
    }

    pub(crate) enum Action {
        Add(crate::Amount),
        Sub(crate::Amount),
        Create(crate::CurrencyId),
        /// Does not handle transference before deletion
        Remove,
    }
}

macro_rules! sub_action {
    ($name: ident -> $key: ident : $Key: ty;  { $create: ident, $remove: ident, $add: ident, $sub: ident }) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum $name {
            $create {
                $key: $Key,
                currency: crate::CurrencyId,
            },
            /// Does not handle transference before deletion
            $remove {
                $key: $Key,
            },
            $add {
                $key: $Key,
                amount: crate::Amount,
            },
            $sub {
                $key: $Key,
                amount: crate::Amount,
            },
        }
    };
}

sub_action!(WalletOperation -> wallet_id: WalletId; { Create, Delete, Deposit, Deduct });
sub_action!(DebtOperation -> debt_id: DebtId; { Incur, Forget, Accumulate, Amortize });

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "group", rename_all = "snake_case")]
pub enum Operation {
    Wallet(WalletOperation),
    Loan(DebtOperation),
    Debt(DebtOperation),
}

#[derive(Debug)]
pub enum Error {
    Wallet(money_record::Error),
    Debt(money_record::Error),
    Loan(money_record::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn print_debt_err(
            e: &money_record::Error,
            f: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            match e {
                money_record::Error::NotFound => write!(f, "debt not found"),
                money_record::Error::CannotSub => write!(f, "debt amortize overflow"),
                money_record::Error::AlreadyExists => write!(f, "debt already exists"),
            }
        }

        match self {
            Error::Wallet(e) => match e {
                money_record::Error::NotFound => write!(f, "wallet not found"),
                money_record::Error::CannotSub => write!(f, "cannot deduct from wallet"),
                money_record::Error::AlreadyExists => write!(f, "wallet already exists"),
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
        fn extract_action(event: DebtOperation) -> (DebtId, money_record::Action) {
            match event {
                DebtOperation::Incur { debt_id, currency } => {
                    (debt_id, money_record::Action::Create(currency))
                }
                DebtOperation::Forget { debt_id } => (debt_id, money_record::Action::Remove),
                DebtOperation::Accumulate { debt_id, amount } => {
                    (debt_id, money_record::Action::Add(amount))
                }
                DebtOperation::Amortize { debt_id, amount } => {
                    (debt_id, money_record::Action::Sub(amount))
                }
            }
        }

        match event {
            Operation::Wallet(event) => {
                let (wallet_id, action) = match event {
                    WalletOperation::Create {
                        wallet_id,
                        currency,
                    } => (wallet_id, money_record::Action::Create(currency)),
                    WalletOperation::Delete { wallet_id } => {
                        (wallet_id, money_record::Action::Remove)
                    }
                    WalletOperation::Deposit { wallet_id, amount } => {
                        (wallet_id, money_record::Action::Add(amount))
                    }
                    WalletOperation::Deduct { wallet_id, amount } => {
                        (wallet_id, money_record::Action::Sub(amount))
                    }
                };

                self.wallets.apply(wallet_id, action).map_err(Error::Wallet)
            }

            Operation::Debt(event) => {
                let (debt_id, action) = extract_action(event);
                self.debts.apply(debt_id, action).map_err(Error::Debt)
            }
            Operation::Loan(event) => {
                let (debt_id, action) = extract_action(event);
                self.loans.apply(debt_id, action).map_err(Error::Loan)
            }
        }
    }
}
