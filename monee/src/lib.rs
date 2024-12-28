pub mod backoffice;
pub mod host;
pub mod reports;
pub mod shared;
pub mod self_config;
pub mod nodes;

pub mod prelude {
    pub use crate::shared::domain::context::AppContext;
    pub use crate::shared::infrastructure::errors::AppError;
    pub use crate::shared::infrastructure::errors::InfrastructureError;

    pub use cream::context::Context;
    #[allow(unused_imports)]
    pub(crate) use crate::iprelude::*;
}

/// Private prelude
pub(crate) mod iprelude {
    use crate::{
        prelude::{AppError, InfrastructureError},
        shared::domain::errors::UniqueSaveError,
    };

    pub use cream::context::FromContext;

    pub trait CatchInfra {
        type Output;
        fn catch_infra(self) -> Result<Self::Output, InfrastructureError>;
    }

    impl<T> CatchInfra for Result<T, surrealdb::Error> {
        type Output = T;
        fn catch_infra(self) -> Result<Self::Output, InfrastructureError> {
            self.map_err(Into::into)
        }
    }

    impl<T, E> CatchInfra for Result<T, AppError<E>> {
        type Output = Result<T, E>;
        fn catch_infra(self) -> Result<Self::Output, InfrastructureError> {
            match self {
                Ok(t) => Ok(Ok(t)),
                Err(AppError::App(e)) => Ok(Err(e)),
                Err(AppError::Infrastructure(e)) => Err(e),
            }
        }
    }

    pub trait CatchApp<AE> {
        type Output;
        fn catch_app(self) -> Result<Self::Output, AppError<AE>>;
    }

    impl<T> CatchApp<UniqueSaveError> for Result<T, surrealdb::Error> {
        type Output = T;
        fn catch_app(self) -> Result<Self::Output, AppError<UniqueSaveError>> {
            self.map_err(Into::into)
        }
    }

    pub trait MapResponse<O, E> {
        fn map_response(self) -> Result<O, E>;
    }

    impl<E> MapResponse<(), E> for Result<surrealdb::Response, E> {
        fn map_response(self) -> Result<(), E> {
            self.map(|_| ())
        }
    }
}
