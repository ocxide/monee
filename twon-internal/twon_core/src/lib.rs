use std::{
    collections::HashMap,
    ops::{AddAssign, Sub, SubAssign},
    str::FromStr,
};

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    wallets: HashMap<WalletId, Wallet>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    balance: Balance,
    currency: CurrencyId,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WalletId(u32);

impl FromStr for WalletId {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CurrencyId(u32);

impl CurrencyId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Balance(u64);

impl Balance {
    pub const fn checked_sub(self, rhs: Balance) -> Option<u64> {
        self.0.checked_sub(rhs.0)
    }
}

#[derive(Debug)]
pub enum FromStrError {
    /// Has more than 4 decimals
    MaxDecimal,
    /// Contains too many commas or dots
    InvalidDecimal,
    /// Is over u64::MAX / MULTIPLIER
    TooBig,
    /// Is not a number
    InvalidNumber,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxDecimal => write!(f, "Has more than 4 decimals"),
            Self::InvalidDecimal => write!(f, "Contains too many commas or dots"),
            Self::TooBig => write!(f, "Is over u64::MAX / MULTIPLIER"),
            Self::InvalidNumber => write!(f, "Is not a number"),
        }
    }
}

impl std::error::Error for FromStrError {}

const DECIMALS: u32 = 4;
const MULTIPLIER: u64 = 10_u64.pow(DECIMALS);

impl FromStr for Balance {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(&['.', ',']);

        let Some(integer_str) = split.next() else {
            return Err(FromStrError::InvalidNumber);
        };

        let integer_part = match integer_str {
            "" => 0,
            _ => integer_str
                .parse::<u64>()
                .map_err(|_| FromStrError::InvalidNumber)?,
        };

        let real = integer_part
            .checked_mul(MULTIPLIER)
            .ok_or(FromStrError::TooBig)?;

        let Some(decimal_str) = split.next() else {
            return Ok(Self(integer_part * MULTIPLIER));
        };

        if split.next().is_some() {
            return Err(FromStrError::InvalidDecimal);
        }

        if decimal_str.is_empty() {
            return Err(FromStrError::InvalidNumber);
        }

        if decimal_str.len() > DECIMALS as usize {
            return Err(FromStrError::MaxDecimal);
        }

        let decimal_part = decimal_str
            .parse::<u64>()
            .map_err(|_| FromStrError::InvalidNumber)?;

        Ok(Self(real + decimal_part))
    }
}

impl SubAssign for Balance {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sub for Balance {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Balance {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Deposit {
        wallet_id: WalletId,
        amount: Balance,
    },
    Deduct {
        wallet_id: WalletId,
        amount: Balance,
    },
    CreateWallet {
        wallet_id: WalletId,
        currency: CurrencyId,
    },
    ///  Does not handle transference before deletion
    DeleteWallet { wallet_id: WalletId },
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
                        balance: Balance(0),
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
