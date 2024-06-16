mod amount;

/// Generic simple, understandable ID with custom length
mod tiny_id;

use std::collections::HashMap;

pub use amount::Amount;
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
    use std::str::FromStr;

    type Id = crate::tiny_id::TinyId<4>;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
    pub struct WalletId(Id);

    impl WalletId {
        pub fn new() -> Self {
            Self(Id::new())
        }
    }

    impl Default for WalletId {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FromStr for WalletId {
        type Err = <Id as FromStr>::Err;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(s.parse()?))
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    wallets: HashMap<WalletId, Wallet>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    balance: Amount,
    currency: CurrencyId,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CurrencyId(u32);

impl CurrencyId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Deposit {
        wallet_id: WalletId,
        amount: Amount,
    },
    Deduct {
        wallet_id: WalletId,
        amount: Amount,
    },
    CreateWallet {
        wallet_id: WalletId,
        currency: CurrencyId,
    },
    ///  Does not handle transference before deletion
    DeleteWallet {
        wallet_id: WalletId,
    },
}

#[derive(Debug)]
pub enum Error {
    WalletAlreadyExists,
    WalletNotFound,
    CannotDeduct,
}

impl Snapshot {
    pub fn apply(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Deposit {
                wallet_id: id,
                amount,
            } => {
                if let Some(wallet) = self.wallets.get_mut(&id) {
                    wallet.balance += amount;
                    Ok(())
                } else {
                    Err(Error::WalletNotFound)
                }
            }
            Event::Deduct {
                wallet_id: id,
                amount,
            } => {
                if let Some(wallet) = self.wallets.get_mut(&id) {
                    wallet
                        .balance
                        .checked_sub(amount)
                        .ok_or(Error::CannotDeduct)?;

                    Ok(())
                } else {
                    Err(Error::WalletNotFound)
                }
            }
            Event::CreateWallet {
                wallet_id: id,
                currency,
            } => {
                if self.wallets.contains_key(&id) {
                    return Err(Error::WalletAlreadyExists);
                }

                self.wallets.insert(
                    id,
                    Wallet {
                        balance: Amount::default(),
                        currency,
                    },
                );
                Ok(())
            }
            Event::DeleteWallet { wallet_id: id } => {
                self.wallets.remove(&id);
                Ok(())
            }
        }
    }
}
