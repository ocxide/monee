use std::ops::{AddAssign, Sub, SubAssign};

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Amount(u64);

impl Amount {
    pub const fn checked_sub(self, rhs: Amount) -> Option<u64> {
        self.0.checked_sub(rhs.0)
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

pub mod from_str {
    use std::str::FromStr;

    use super::Amount;

    #[derive(Debug)]
    pub enum Error {
        /// Has more than 4 decimals
        MaxDecimal,
        /// Contains too many commas or dots
        InvalidDecimal,
        /// Is over u64::MAX / MULTIPLIER
        TooBig,
        /// Is not a number
        InvalidNumber,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::MaxDecimal => write!(f, "Has more than 4 decimals"),
                Self::InvalidDecimal => write!(f, "Contains too many commas or dots"),
                Self::TooBig => write!(f, "Is over u64::MAX / MULTIPLIER"),
                Self::InvalidNumber => write!(f, "Is not a number"),
            }
        }
    }

    impl std::error::Error for Error {}

    const DECIMALS: u32 = 4;
    const MULTIPLIER: u64 = 10_u64.pow(DECIMALS);

    impl FromStr for Amount {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut split = s.split(&['.', ',']);

            let Some(integer_str) = split.next() else {
                return Err(Error::InvalidNumber);
            };

            let integer_part = match integer_str {
                "" => 0,
                _ => integer_str
                    .parse::<u64>()
                    .map_err(|_| Error::InvalidNumber)?,
            };

            let real = integer_part.checked_mul(MULTIPLIER).ok_or(Error::TooBig)?;

            let Some(decimal_str) = split.next() else {
                return Ok(Self(integer_part * MULTIPLIER));
            };

            if split.next().is_some() {
                return Err(Error::InvalidDecimal);
            }

            if decimal_str.is_empty() {
                return Err(Error::InvalidNumber);
            }

            if decimal_str.len() > DECIMALS as usize {
                return Err(Error::MaxDecimal);
            }

            let decimal_part = decimal_str
                .parse::<u64>()
                .map_err(|_| Error::InvalidNumber)?;

            Ok(Self(real + decimal_part))
        }
    }
}

