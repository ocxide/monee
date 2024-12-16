pub mod save_one {
    use cream::context::FromContext;
    use monee_core::CurrencyId;

    use crate::{
        backoffice::currencies::domain::{currency::Currency, repository::Repository},
        prelude::AppError,
        shared::domain::{context::AppContext, errors::UniqueSaveError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
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
    use cream::context::FromContext;
    use monee_core::CurrencyId;

    use crate::{
        backoffice::currencies::domain::{currency_code::CurrencyCode, repository::Repository},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct CodeResolve {
        repository: Box<dyn Repository>,
    }

    impl CodeResolve {
        pub async fn run(
            &self,
            code: CurrencyCode,
        ) -> Result<Option<CurrencyId>, InfrastructureError> {
            self.repository.code_resolve(code).await
        }
    }
}

pub mod get_all {
    use cream::context::FromContext;
    use monee_core::CurrencyId;

    use crate::{
        backoffice::currencies::domain::{currency::Currency, repository::Repository},
        prelude::{AppContext, InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetAll {
        repository: Box<dyn Repository>,
    }

    impl GetAll {
        pub async fn run(&self) -> Result<Vec<(CurrencyId, Currency)>, InfrastructureError> {
            self.repository.get_all().await
        }
    }
}
