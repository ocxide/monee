mod entity;
pub use entity::*;

mod catch {
    use axum::http::StatusCode;
    use cream::context::Context;
    use monee::{
        prelude::{AppContext, AppError, InfrastructureError},
        shared::application::logging::LogService,
    };

    pub trait CatchInfra<T>: Sized {
        type Output;
        fn catch_infra(self, ctx: &AppContext) -> Result<Self::Output, StatusCode> {
            match self.get_infra() {
                Ok(t) => Ok(t),
                Err(e) => {
                    let logger: LogService = ctx.provide();
                    logger.error(e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        fn get_infra(self) -> Result<Self::Output, InfrastructureError>;
    }

    impl<T> CatchInfra<T> for Result<T, InfrastructureError> {
        type Output = T;

        fn get_infra(self) -> Result<Self::Output, InfrastructureError> {
            self
        }
    }

    impl<T, E> CatchInfra<T> for Result<T, AppError<E>> {
        type Output = Result<T, E>;

        fn get_infra(self) -> Result<Self::Output, InfrastructureError> {
            match self {
                Ok(t) => Ok(Ok(t)),
                Err(AppError::App(e)) => Ok(Err(e)),
                Err(AppError::Infrastructure(e)) => Err(e),
            }
        }
    }
}

pub use catch::*;

mod into_json {
    use axum::Json;

    use super::Entity;

    pub trait IntoJson<K, T> {
        type Output;

        fn into_json(self) -> Self::Output;
    }

    impl<K, T, E> IntoJson<K, T> for Result<Vec<(K, T)>, E> {
        type Output = Result<Json<Vec<Entity<K, T>>>, E>;

        fn into_json(self) -> Self::Output {
            self.map(|currencies| {
                Json(currencies.into_iter().map(Entity::from).collect::<Vec<_>>())
            })
        }
    }
}

pub use into_json::*;

mod catch_app {
    use axum::http::StatusCode;
    use monee::{
        prelude::{AppError, InfrastructureError},
        shared::domain::errors::UniqueSaveError,
    };

    pub trait CatchApp {
        fn catch_app(self) -> Result<StatusCode, InfrastructureError>;
    }

    impl CatchApp for Result<(), AppError<UniqueSaveError>> {
        fn catch_app(self) -> Result<StatusCode, InfrastructureError> {
            match self {
                Ok(_) => Ok(StatusCode::OK),
                Err(AppError::Infrastructure(e)) => Err(e),
                Err(AppError::App(UniqueSaveError::AlreadyExists(_))) => Ok(StatusCode::CONFLICT),
            }
        }
    }
}

pub use catch_app::*;
