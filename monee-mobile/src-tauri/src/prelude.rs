pub use monee::prelude::*;
use monee::shared::infrastructure::errors::UnspecifiedError;

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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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

impl<T, E> CatchInfra for Result<T, AppError<E>> {
    type Output = Result<T, E>;
    fn get_error(self) -> Result<Self::Output, InfrastructureError> {
        match self {
            Ok(v) => Ok(Ok(v)),
            Err(AppError::App(e)) => Ok(Err(e)),
            Err(AppError::Infrastructure(e)) => Err(e),
        }
    }
}

pub trait CatchToInfra {
    type Output;
    fn catch_to_infra(self) -> Result<Self::Output, InfrastructureError>;
}

impl<T> CatchToInfra for tauri_plugin_http::reqwest::Result<T> {
    type Output = T;
    fn catch_to_infra(self) -> Result<Self::Output, InfrastructureError> {
        use tauri::http::StatusCode;
        self.map_err(|e| {
            if let Some(StatusCode::UNAUTHORIZED) = e.status() {
                InfrastructureError::Auth
            } else {
                InfrastructureError::Unspecified(UnspecifiedError::new(e))
            }
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum MoneeError<E> {
    App(E),
    Internal(InternalError),
}

impl<E> From<InternalError> for MoneeError<E> {
    fn from(value: InternalError) -> Self {
        Self::Internal(value)
    }
}

impl<E> From<AppError<E>> for MoneeError<E> {
    fn from(value: AppError<E>) -> Self {
        match value {
            AppError::App(e) => Self::App(e),
            AppError::Infrastructure(e) => Self::Internal(InternalError::from_ref(&e)),
        }
    }
}
