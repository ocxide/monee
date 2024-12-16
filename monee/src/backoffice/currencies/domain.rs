pub use monee_types::backoffice::currencies::*;
pub mod repository {
    use monee_core::CurrencyId;

    use crate::{
        prelude::AppError,
        shared::{domain::errors::UniqueSaveError, infrastructure::errors::InfrastructureError},
    };

    use super::{currency::Currency, currency_code::CurrencyCode};

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(
            &self,
            id: CurrencyId,
            currency: Currency,
        ) -> Result<(), AppError<UniqueSaveError>>;

        async fn code_resolve(
            &self,
            code: CurrencyCode,
        ) -> Result<Option<CurrencyId>, InfrastructureError>;

        async fn get_all(&self) -> Result<Vec<(CurrencyId, Currency)>, InfrastructureError>;
    }
}

