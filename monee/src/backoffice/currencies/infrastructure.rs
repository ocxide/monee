pub mod repository {
    use cream::context::FromContext;
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

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn save(
            &self,
            id: CurrencyId,
            currency: Currency,
        ) -> Result<(), AppError<UniqueSaveError>> {
            let response = self
                .0
                .query("CREATE type::thing('currency', ) SET name = , symbol = , code = ")
                .bind(("id", id))
                .bind(currency)
                .await
                .map_err(InfrastructureError::from)?
                .check();

            response.into_app_result()
        }

        async fn code_resolve(
            &self,
            code: &CurrencyCode,
        ) -> Result<Option<CurrencyId>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM currency WHERE code = ")
                .bind(("code", code))
                .await?;

            let id: Option<Entity<CurrencyId, ()>> = response.take(0)?;
            Ok(id.map(|e| e.0))
        }

        async fn get_all(&self) -> Result<Vec<(CurrencyId, Currency)>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id, name, symbol, code FROM currency")
                .await?;

            let entities: Vec<Entity<CurrencyId, Currency>> = response.take(0)?;
            Ok(entities.into_iter().map(|e| (e.0, e.1)).collect())
        }
    }
}
