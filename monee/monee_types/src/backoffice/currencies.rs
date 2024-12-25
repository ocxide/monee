pub mod currency {
    use super::{
        currency_code::CurrencyCode, currency_name::CurrencyName, currency_symbol::CurrencySymbol,
    };

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct Currency {
        pub name: CurrencyName,
        pub symbol: CurrencySymbol,
        pub code: CurrencyCode,
    }
}

pub mod currency_symbol {
    use std::{fmt::Display, str::FromStr};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct CurrencySymbol(String);

    impl Display for CurrencySymbol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Currency symbol must have at least one digit")]
        InvalidLength,

        #[error("Currency symbol must not have any whitespace or number")]
        InvalidChar,
    }

    impl FromStr for CurrencySymbol {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.is_empty() {
                return Err(Error::InvalidLength);
            }

            if s.chars().any(|c| c.is_numeric() || c.is_whitespace()) {
                return Err(Error::InvalidChar);
            }

            Ok(Self(s.to_owned()))
        }
    }
}

pub mod currency_name {
    use std::fmt::Display;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct CurrencyName(String);

    impl Display for CurrencyName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<String> for CurrencyName {
        fn from(name: String) -> Self {
            Self(name)
        }
    }
}

pub mod currency_code {
    use std::{fmt::Display, str::FromStr};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CurrencyCode(Inner);

    impl Display for CurrencyCode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.as_ref().fmt(f)
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Currency code must have 3 characters")]
        InvalidLength,
        #[error("Currency code must be alphabetic")]
        NotAlphabetic,
    }

    pub const CODE_LENGTH: usize = 3;
    type Inner = [u8; CODE_LENGTH];

    impl FromStr for CurrencyCode {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != CODE_LENGTH {
                return Err(Error::InvalidLength);
            }

            let mut buf: Inner = Default::default();
            for (buf_c, c) in std::iter::zip(buf.iter_mut(), s.chars()) {
                if !c.is_alphabetic() {
                    return Err(Error::NotAlphabetic);
                }

                let byte: u8 = c
                    .to_ascii_uppercase()
                    .try_into()
                    .map_err(|_| Error::NotAlphabetic)?;
                *buf_c = byte;
            }

            Ok(Self(buf))
        }
    }

    impl AsRef<str> for CurrencyCode {
        fn as_ref(&self) -> &str {
            unsafe { std::str::from_utf8_unchecked(&self.0) }
        }
    }

    impl PartialEq<str> for CurrencyCode {
        fn eq(&self, other: &str) -> bool {
            self.as_ref() == other
        }
    }

    impl serde::Serialize for CurrencyCode {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.as_ref().serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for CurrencyCode {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // TODO: this can be more efficient, but I dont care now
            let s = String::deserialize(deserializer)?;
            s.parse().map_err(serde::de::Error::custom)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn serializes_as_str() {
            let code: CurrencyCode = "ABC".parse().unwrap();
            assert_eq!(
                serde_json::to_value(&code).unwrap(),
                serde_json::Value::String("ABC".to_owned())
            );
        }

        #[test]
        fn converts_to_uppercase() {
            let code: CurrencyCode = "abc".parse().unwrap();
            assert_eq!(&code, "ABC");
        }

        #[test]
        fn prints_as_str() {
            let code: CurrencyCode = "ABC".parse().unwrap();
            assert_eq!(&code.to_string(), "ABC");
        }
    }
}

pub mod currency_created {
    use cream_events_core::DomainEvent;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct CurrencyCreated {
        pub id: monee_core::CurrencyId,
    }

    impl DomainEvent for CurrencyCreated {
        fn name(&self) -> &'static str {
            "backoffice.currencies.created"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
