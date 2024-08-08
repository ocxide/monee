pub mod domain {
    pub mod repository {
        use cream::context::FromContext;
        use monee_core::{currency::Currency, CurrencyId};

        use crate::shared::{
            domain::context::AppContext,
            infrastructure::errors::{UniqueSaveError, UnspecifiedError},
        };

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: CurrencyId, currency: Currency)
                -> Result<(), UniqueSaveError>;
            async fn code_resolve(
                &self,
                code: &str,
            ) -> Result<Option<CurrencyId>, UnspecifiedError>;
        }

        impl<C: AppContext> FromContext<C> for Box<dyn Repository> {
            fn from_context(context: &C) -> Self {
                context.backoffice_currencies_repository()
            }
        }
    }
}

pub mod application {
    pub mod save_one {
        use cream::context::FromContext;
        use monee_core::{currency::Currency, CurrencyId};

        use crate::{
            backoffice::currencies::domain::repository::Repository,
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        #[derive(FromContext)]
        #[from_context(C: AppContext)]
        pub struct SaveOne {
            repository: Box<dyn Repository>,
        }

        impl SaveOne {
            pub async fn run(&self, currency: Currency) -> Result<(), Error> {
                self.repository
                    .save(CurrencyId::new(), currency)
                    .await
                    .map_err(Into::into)
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            Unspecified(#[from] UnspecifiedError),

            #[error("Currency already exists")]
            RepeatedCode,
        }

        impl From<UniqueSaveError> for Error {
            fn from(err: UniqueSaveError) -> Self {
                match err {
                    UniqueSaveError::AlreadyExists => Self::RepeatedCode,
                    UniqueSaveError::Unspecified(err) => Self::Unspecified(err),
                }
            }
        }
    }

    pub mod code_resolve {
        use cream::context::FromContext;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::UnspecifiedError},
        };

        #[derive(FromContext)]
        #[from_context(C: AppContext)]
        pub struct CodeResolve {
            repository: Box<dyn Repository>,
        }

        impl CodeResolve {
            pub async fn run(&self, code: &str) -> Result<Option<CurrencyId>, UnspecifiedError> {
                self.repository.code_resolve(code).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::{currency::Currency, CurrencyId};

        use crate::{
            backoffice::currencies::domain::repository::Repository,
            shared::infrastructure::{
                database::{Connection, Entity},
                errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        pub struct SurrealRepository(Connection);
        impl SurrealRepository {
            pub(crate) fn new(
                clone: surrealdb::Surreal<surrealdb::engine::remote::ws::Client>,
            ) -> Self {
                Self(clone)
            }
        }

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(
                &self,
                id: CurrencyId,
                currency: Currency,
            ) -> Result<(), UniqueSaveError> {
                let response = self.0
                .query(
                    "CREATE ONLY type::thing('currency', $id) SET name = $name, symbol = $symbol, code = $code",
                )
                .bind(("id", id))
                .bind(("name", currency.name))
                .bind(("symbol", currency.symbol))
                .bind(("code", currency.code))
                .await.map_err(UnspecifiedError::from)?
                .check();

                match response {
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(UniqueSaveError::AlreadyExists),
                    Err(e) => Err(UniqueSaveError::Unspecified(e.into())),
                    Ok(_) => Ok(()),
                }
            }

            async fn code_resolve(
                &self,
                code: &str,
            ) -> Result<Option<CurrencyId>, UnspecifiedError> {
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
