pub mod wallet {
    use super::wallet_name::WalletName;

    pub struct Wallet {
        pub currency_id: monee_core::CurrencyId,
        pub name: WalletName,
        pub description: String,
    }
}

pub mod wallet_name {
    use std::{fmt::Display, str::FromStr};

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct WalletName(String);

    impl Display for WalletName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Invalid character: {0:?}")]
        InvalidCharacter(char),
    }

    impl TryFrom<String> for WalletName {
        type Error = Error;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            match value
                .chars()
                .find(|c| !matches!(*c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'))
            {
                Some(c) => Err(Error::InvalidCharacter(c)),
                None => Ok(Self(value)),
            }
        }
    }

    impl FromStr for WalletName {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Self::try_from(s.to_string())
        }
    }
}

pub mod wallet_created {
    use cream_events_core::DomainEvent;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct WalletCreated {
        pub id: monee_core::WalletId,
        pub currency_id: monee_core::CurrencyId,
    }

    impl DomainEvent for WalletCreated {
        fn name(&self) -> &'static str {
            "backoffice.wallets.created"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
