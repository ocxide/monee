mod entity;
pub use entity::*;

mod catch {
    use axum::http::StatusCode;
    use cream::context::Context;
    use monee::{
        prelude::{AppContext, InfrastructureError},
        shared::application::logging::LogService,
    };

    pub trait CatchInfra<T> {
        fn catch_infra(self, ctx: &AppContext) -> Result<T, StatusCode>;
    }

    impl<T> CatchInfra<T> for Result<T, InfrastructureError> {
        fn catch_infra(self, ctx: &AppContext) -> Result<T, StatusCode> {
            self.map_err(|e| {
                let logger: LogService = ctx.provide();
                logger.error(e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
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
