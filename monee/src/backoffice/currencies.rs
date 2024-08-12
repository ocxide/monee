pub mod domain {
    pub mod currency {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct Currency {
            pub name: String,
            pub symbol: String,
            pub code: String,
        }
    }

    pub mod repository {
        use monee_core::CurrencyId;

        use crate::shared::{
            domain::errors::UniqueSaveStatus, infrastructure::errors::InfrastructureError,
        };

        use super::currency::Currency;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                id: CurrencyId,
                currency: Currency,
            ) -> Result<UniqueSaveStatus, InfrastructureError>;
            async fn code_resolve(
                &self,
                code: &str,
            ) -> Result<Option<CurrencyId>, InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod save_one {
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::{currency::Currency, repository::Repository},
            shared::{
                domain::{context::AppContext, errors::UniqueSaveStatus},
                infrastructure::errors::InfrastructureError,
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct SaveOne {
            repository: Box<dyn Repository>,
        }

        impl SaveOne {
            pub async fn run(
                &self,
                currency: Currency,
            ) -> Result<UniqueSaveStatus, InfrastructureError> {
                self.repository.save(CurrencyId::new(), currency).await
            }
        }
    }

    pub mod code_resolve {
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct CodeResolve {
            repository: Box<dyn Repository>,
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
        use cream::context::ContextProvide;
        use monee_core::CurrencyId;

        use crate::{
            backoffice::currencies::domain::{currency::Currency, repository::Repository},
            shared::{
                domain::{
                    context::DbContext,
                    errors::{IntoDomainResult, UniqueSaveStatus},
                },
                infrastructure::{
                    database::{Connection, Entity},
                    errors::InfrastructureError,
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
            ) -> Result<UniqueSaveStatus, InfrastructureError> {
                let response = self.0
                .query(
                    "CREATE ONLY type::thing('currency', $id) SET name = $name, symbol = $symbol, code = $code",
                )
                .bind(("id", id))
                .bind(("name", currency.name))
                .bind(("symbol", currency.symbol))
                .bind(("code", currency.code))
                .await?
                .check();

                response.into_domain_result()
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
