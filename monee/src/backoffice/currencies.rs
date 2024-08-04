pub mod domain {
    pub mod repository {
        use monee_core::{currency::Currency, CurrencyId};

        use crate::shared::errors::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: CurrencyId, currency: Currency) -> Result<(), SaveError>;
            async fn code_resolve(
                &self,
                code: &str,
            ) -> Result<Option<CurrencyId>, InfrastructureError>;
        }

        #[derive(thiserror::Error, Debug)]
        pub enum SaveError {
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),

            #[error("Currency already exists")]
            RepeatedCode,
        }
    }
}

pub mod application {
    pub mod save_one {
        use cream::from_context::FromContext;
        use monee_core::{currency::Currency, CurrencyId};

        use crate::{
            backoffice::currencies::domain::repository::{Repository, SaveError},
            shared::domain::context::AppContext,
        };

        pub struct SaveOne {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for SaveOne {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_currencies_repository(),
                }
            }
        }

        impl SaveOne {
            pub async fn run(&self, currency: Currency) -> Result<(), SaveError> {
                self.repository.save(CurrencyId::new(), currency).await
            }
        }
    }

    pub mod code_resolve {
        use cream::from_context::FromContext;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::repository::Repository,
            shared::{domain::context::AppContext, errors::InfrastructureError},
        };

        pub struct CodeResolve {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for CodeResolve {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_currencies_repository(),
                }
            }
        }

        impl CodeResolve {
            pub async fn run(&self, code: &str) -> Result<Option<CurrencyId>, InfrastructureError> {
                self.repository.code_resolve(code).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::{currency::Currency, CurrencyId};

        use crate::{
            backoffice::currencies::domain::repository::{Repository, SaveError},
            shared::{
                errors::InfrastructureError,
                infrastructure::database::{Connection, Entity},
            },
        };

        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: CurrencyId, currency: Currency) -> Result<(), SaveError> {
                let response = self.0
                .query(
                    "CREATE ONLY type::thing('currency', $id) SET name = $name, symbol = $symbol, code = $code",
                )
                .bind(("id", id))
                .bind(("name", currency.name))
                .bind(("symbol", currency.symbol))
                .bind(("code", currency.code))
                .await.map_err(InfrastructureError::new)?
                .check();

                match response {
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(SaveError::RepeatedCode),
                    Err(e) => Err(InfrastructureError::new(e).into()),
                    Ok(_) => Ok(()),
                }
            }

            async fn code_resolve(
                &self,
                code: &str,
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
