use std::ops::{AddAssign, Sub, SubAssign};

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Amount(u64);

impl Amount {
    pub const fn checked_sub(self, rhs: Amount) -> Option<Amount> {
        match self.0.checked_sub(rhs.0) {
            Some(amount) => Some(Amount(amount)),
            None => None,
        }
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

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let int = self.0 / MULTIPLIER;
        write!(f, "{}", int)?;

        let mut floating = self.0 - (int * MULTIPLIER);
        if floating == 0 {
            return Ok(());
        }

        write!(f, ".")?;

        while floating != 0 {
            floating *= 10;
            write!(f, "{}", floating / MULTIPLIER)?;
            floating %= MULTIPLIER;
        }

        Ok(())
    }
}

const DECIMALS: u32 = 4;
const MULTIPLIER: u64 = 10_u64.pow(DECIMALS);

pub mod from_str {
    use std::str::FromStr;

    use super::{Amount, DECIMALS, MULTIPLIER};

    #[derive(Debug, PartialEq)]
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

    impl FromStr for Amount {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut split = s.split(&['.', ',']);

            let int_part = split.next();
            let decimal_part = split.next();

            if split.next().is_some() {
                return Err(Error::InvalidDecimal);
            }

            let int = match int_part {
                Some("") => Ok(0),
                Some(int_part) => int_part.parse::<u64>().map_err(|_| Error::InvalidNumber),
                None => Err(Error::InvalidNumber),
            }?
            .checked_mul(MULTIPLIER)
            .ok_or(Error::TooBig)?;

            let (parsed_decimal, len) = match decimal_part {
                Some("") => Err(Error::InvalidNumber),
                Some(decimal_part) if decimal_part.len() > DECIMALS as usize => {
                    Err(Error::MaxDecimal)
                }
                Some(decimal_part) => match decimal_part.parse::<u64>() {
                    Ok(parsed_decimal) => Ok((parsed_decimal, decimal_part.len())),
                    Err(_) => Err(Error::InvalidNumber),
                },
                None => Ok((0, 0)),
            }?;

            let decimal = parsed_decimal * 10u64.pow(DECIMALS - len as u32);

            Ok(Self(int + decimal))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_normal() {
        assert_eq!(Amount(1234567).to_string(), "123.4567");
    }

    #[test]
    fn display_zero() {
        assert_eq!(Amount(0).to_string(), "0");
    }

    #[test]
    fn donot_display_decimal() {
        assert_eq!(Amount(10000).to_string(), "1");
    }

    #[test]
    fn donot_display_trailing() {
        assert_eq!(Amount(1234500).to_string(), "123.45");
    }

    #[test]
    fn displays_intermidiat_zeros() {
        assert_eq!(Amount(1230045).to_string(), "123.0045");
    }

    #[test]
    fn from_str() {
        assert_eq!(Amount(1234567), "123.4567".parse().unwrap());
    }

    #[test]
    fn from_str_21_90() {
        assert_eq!("21.90".parse::<Amount>(), Ok(Amount(219000)));
    }
}
