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
