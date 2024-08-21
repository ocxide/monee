pub mod domain {
    pub mod currency {
        use super::{
            currency_code::CurrencyCode, currency_name::CurrencyName,
            currency_symbol::CurrencySymbol,
        };

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct Currency {
            pub name: CurrencyName,
            pub symbol: CurrencySymbol,
            pub code: CurrencyCode,
        }
    }

    pub mod repository {
        use monee_core::CurrencyId;

        use crate::{
            prelude::AppError,
            shared::{
                domain::errors::UniqueSaveError, infrastructure::errors::InfrastructureError,
            },
        };

        use super::{currency::Currency, currency_code::CurrencyCode};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                id: CurrencyId,
                currency: Currency,
            ) -> Result<(), AppError<UniqueSaveError>>;

            async fn code_resolve(
                &self,
                code: &CurrencyCode,
            ) -> Result<Option<CurrencyId>, InfrastructureError>;
        }
    }

    pub mod currency_symbol {
        use std::{fmt::Display, str::FromStr};

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        use std::{char, fmt::Display, str::FromStr};

        #[derive(Debug, Clone)]
        pub struct CurrencyCode([char; CODE_LENGTH]);

        impl Display for CurrencyCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for c in self.0.iter() {
                    write!(f, "{}", c)?;
                }

                Ok(())
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

        fn extract_chars(s: &str) -> Option<[char; CODE_LENGTH]> {
            let mut chars = s.chars();
            let slice = [chars.next()?, chars.next()?, chars.next()?];
            if chars.next().is_some() {
                return None;
            }

            Some(slice)
        }

        impl FromStr for CurrencyCode {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let arr = extract_chars(s).ok_or(Error::InvalidLength)?;

                if arr.iter().copied().all(char::is_alphabetic) {
                    Ok(Self(arr))
                } else {
                    Err(Error::NotAlphabetic)
                }
            }
        }

        impl serde::Serialize for CurrencyCode {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let arr = [self.0[0] as u8, self.0[1] as u8, self.0[2] as u8];
                unsafe { std::str::from_utf8_unchecked(&arr) }.serialize(serializer)
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
        }
    }
}

pub mod application {
    pub mod save_one {
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::{currency::Currency, repository::Repository},
            prelude::AppError,
            shared::domain::{context::AppContext, errors::UniqueSaveError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct SaveOne {
            repository: Box<dyn Repository>,
        }

        impl SaveOne {
            pub async fn run(&self, currency: Currency) -> Result<(), AppError<UniqueSaveError>> {
                self.repository.save(CurrencyId::new(), currency).await
            }
        }
    }

    pub mod code_resolve {
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::{currency_code::CurrencyCode, repository::Repository},
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct CodeResolve {
            repository: Box<dyn Repository>,
        }

        impl CodeResolve {
            pub async fn run(
                &self,
                code: &CurrencyCode,
            ) -> Result<Option<CurrencyId>, InfrastructureError> {
                self.repository.code_resolve(code).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::{
                currency::Currency, currency_code::CurrencyCode, repository::Repository,
            },
            prelude::AppError,
            shared::{
                domain::{context::DbContext, errors::UniqueSaveError},
                infrastructure::{
                    database::{Connection, Entity},
                    errors::{InfrastructureError, IntoAppResult},
                },
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(
                &self,
                id: CurrencyId,
                currency: Currency,
            ) -> Result<(), AppError<UniqueSaveError>> {
                let response = self.0
                .query(
                    "CREATE type::thing('currency', $id) SET name = $name, symbol = $symbol, code = $code",
                )
                .bind(("id", id))
                .bind(currency)
                .await.map_err(InfrastructureError::from)?
                .check();

                response.into_app_result()
            }

            async fn code_resolve(
                &self,
                code: &CurrencyCode,
            ) -> Result<Option<CurrencyId>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT id FROM currency WHERE code = $code")
                    .bind(("code", code))
                    .await?;

                let id: Option<Entity<CurrencyId, ()>> = response.take(0)?;
                Ok(id.map(|e| e.0))
            }
        }
    }
}
