

pub mod actor {
    use super::{actor_alias::ActorAlias, actor_name::ActorName, actor_type::ActorType};

    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    pub struct Actor {
        pub name: ActorName,
        #[serde(rename = "type")]
        pub actor_type: ActorType,
        pub alias: Option<ActorAlias>,
    }
}

pub mod actor_name {
    use std::fmt::Display;

    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    pub struct ActorName(String);

    impl Display for ActorName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<String> for ActorName {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
}

pub mod actor_alias {
    use std::{fmt::Display, str::FromStr};

    use crate::shared::alias::{from_str::Error, Alias};

    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    pub struct ActorAlias(Alias);

    impl Display for ActorAlias {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl FromStr for ActorAlias {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(Alias::from_str(s)?))
        }
    }
}

pub mod actor_type {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    pub enum ActorType {
        Natural,
        Business,
        FinancialEntity,
    }

    pub mod actor_type_from_str {
        use std::str::FromStr;

        use super::ActorType;

        #[derive(Debug)]
        pub struct Error {}

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "invalid actor type, must be 'natural', 'business', or 'financial_entity'"
                )
            }
        }

        impl std::error::Error for Error {}

        impl FromStr for ActorType {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    "natural" | "n" => Ok(Self::Natural),
                    "business" | "b" => Ok(Self::Business),
                    "financial_entity" | "f" => Ok(Self::FinancialEntity),
                    _ => Err(Error {}),
                }
            }
        }
    }
}
