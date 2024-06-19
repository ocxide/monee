mod amount;

/// Generic simple, understandable ID with custom length
mod tiny_id;

pub use amount::Amount;
pub use currency_id::CurrencyId;
pub use debt_id::DebtId;
pub use wallet_id::WalletId;

pub mod metadata {
    use crate::WalletId;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct WalletMetadata {
        pub id: WalletId,
        pub name: String,
    }
}

mod wallet_id {
    type Id = crate::tiny_id::TinyId<4>;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
    pub struct WalletId(Id);

    crate::id_utils::impl_id!(WalletId, Id);
}

mod currency_id {
    type Id = crate::tiny_id::TinyId<4>;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
    pub struct CurrencyId(Id);

    crate::id_utils::impl_id!(CurrencyId, Id);
}

mod debt_id {
    type Id = crate::tiny_id::TinyId<4>;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
    pub struct DebtId(Id);

    crate::id_utils::impl_id!(DebtId, Id);
}

mod id_utils {
    macro_rules! impl_id {
        ($name:ident, $inner_id:ty) => {
            impl $name {
                pub fn new() -> Self {
                    Self(Id::new())
                }
            }

            impl Default for $name {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl std::str::FromStr for $name {
                type Err = <$inner_id as std::str::FromStr>::Err;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self(s.parse()?))
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
        };
    }

    pub(crate) use impl_id;
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub wallets: money_storage::StorageMap<WalletId>,
    pub debts: money_storage::StorageMap<DebtId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Debt {
    pub amount: Amount,
    pub currency: CurrencyId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    pub balance: Amount,
    pub currency: CurrencyId,
}

mod money_storage {
    use std::collections::HashMap;

    #[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(bound = "K: serde::Serialize + serde::de::DeserializeOwned")]
    pub struct StorageMap<K: Eq + std::hash::Hash>(HashMap<K, MoneyStorage>);

    impl<K: Eq + std::hash::Hash> IntoIterator for StorageMap<K> {
        type Item = <HashMap<K, MoneyStorage> as IntoIterator>::Item;
        type IntoIter = <HashMap<K, MoneyStorage> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    impl<K: Eq + std::hash::Hash + std::ops::Deref> std::ops::Deref for StorageMap<K> {
        type Target = HashMap<K, MoneyStorage>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<K: Eq + std::hash::Hash> StorageMap<K> {
        pub fn apply(&mut self, key: K, action: Action) -> Result<(), Error> {
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
                        storage
                            .balance
                            .checked_sub(amount)
                            .ok_or(Error::CannotSub)?;
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

    pub enum Action {
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

sub_action!(WalletEvent -> wallet_id: WalletId; { Create, Delete, Deposit, Deduct });
sub_action!(DebtEvent -> debt_id: DebtId; { Incur, Forget, Accumulate, Amortize });

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Wallet(WalletEvent),
    Debt(DebtEvent),
}

#[derive(Debug)]
pub enum Error {
    Wallet(money_storage::Error),
    Debt(money_storage::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Wallet(e) => match e {
                money_storage::Error::NotFound => write!(f, "wallet not found"),
                money_storage::Error::CannotSub => write!(f, "cannot deduct from wallet"),
                money_storage::Error::AlreadyExists => write!(f, "wallet already exists"),
            },
            Error::Debt(e) => match e {
                money_storage::Error::NotFound => write!(f, "debt not found"),
                money_storage::Error::CannotSub => write!(f, "debt amortize overflow"),
                money_storage::Error::AlreadyExists => write!(f, "debt already exists"),
            },
        }
    }
}

impl std::error::Error for Error {}

impl Snapshot {
    pub fn apply(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Wallet(event) => {
                let (wallet_id, action) = match event {
                    WalletEvent::Create {
                        wallet_id,
                        currency,
                    } => (wallet_id, money_storage::Action::Create(currency)),
                    WalletEvent::Delete { wallet_id } => (wallet_id, money_storage::Action::Remove),
                    WalletEvent::Deposit { wallet_id, amount } => {
                        (wallet_id, money_storage::Action::Add(amount))
                    }
                    WalletEvent::Deduct { wallet_id, amount } => {
                        (wallet_id, money_storage::Action::Sub(amount))
                    }
                };

                self.wallets.apply(wallet_id, action).map_err(Error::Wallet)
            }

            Event::Debt(event) => {
                let (debt_id, action) = match event {
                    DebtEvent::Incur { debt_id, currency } => {
                        (debt_id, money_storage::Action::Create(currency))
                    }
                    DebtEvent::Forget { debt_id } => (debt_id, money_storage::Action::Remove),
                    DebtEvent::Accumulate { debt_id, amount } => {
                        (debt_id, money_storage::Action::Add(amount))
                    }
                    DebtEvent::Amortize { debt_id, amount } => {
                        (debt_id, money_storage::Action::Sub(amount))
                    }
                };

                self.debts.apply(debt_id, action).map_err(Error::Debt)
            }
        }
    }
}
