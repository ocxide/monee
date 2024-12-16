pub mod repository {
    use cream::context::FromContext;
    use monee_core::CurrencyId;

    use crate::{
        backoffice::currencies::domain::{
            currency::Currency, currency_code::CurrencyCode, repository::Repository,
        },
        iprelude::*,
        prelude::AppError,
        shared::{
            domain::{context::DbContext, errors::UniqueSaveError},
            infrastructure::{
                database::{Connection, Entity},
                errors::InfrastructureError,
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
                .query("CREATE type::thing('currency', $id) SET name = $name, symbol = $symbol, code = $code")
                .bind(("id", id))
                .bind(currency)
                .await
                .catch_infra()?
                .check();

            response.catch_app().map_response()
        }

        async fn code_resolve(
            &self,
            code: CurrencyCode,
        ) -> Result<Option<CurrencyId>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM currency WHERE code = $code")
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
