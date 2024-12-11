pub use monee::prelude::*;

pub trait CatchInfra: Sized {
    type Output;
    fn get_error(self) -> Result<Self::Output, InfrastructureError>;
    fn catch_infra(self, ctx: &AppContext) -> Result<Self::Output, InternalError> {
        self.get_error().map_err(|e| {
            let error = InternalError::from_ref(&e);

            let log: monee::shared::application::logging::LogService = ctx.provide();
            log.error(e);

            error
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum InternalError {
    Auth,
    Unknown,
}

impl InternalError {
    pub fn from_ref(e: &InfrastructureError) -> Self {
        match e {
            InfrastructureError::Auth => Self::Auth,
            _ => Self::Unknown,
        }
    }
}

impl<T> CatchInfra for Result<T, InfrastructureError> {
    type Output = T;
    fn get_error(self) -> Result<Self::Output, InfrastructureError> {
        self
    }
}

