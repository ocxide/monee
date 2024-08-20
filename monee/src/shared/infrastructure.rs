pub mod database;

pub mod errors {
    #[derive(Debug, thiserror::Error)]
    #[error("infrastructure error: {0}")]
    pub struct UnspecifiedError(Box<dyn std::error::Error>);

    #[derive(thiserror::Error, Debug)]
    pub enum InfrastructureError {
        #[error("authentication failed")]
        Auth,
        #[error(transparent)]
        Unspecified(UnspecifiedError),
    }

    impl From<surrealdb::Error> for InfrastructureError {
        fn from(err: surrealdb::Error) -> Self {
            Self::Unspecified(UnspecifiedError(err.into()))
        }
    }

    pub enum AppError<E> {
        App(E),
        Infrastructure(InfrastructureError),
    }

    impl<E> From<InfrastructureError> for AppError<E> {
        fn from(value: InfrastructureError) -> Self {
            Self::Infrastructure(value)
        }
    }

    pub trait IntoAppResult<E> {
        fn into_app_result(self) -> Result<(), AppError<E>>;
    }
}

pub mod logging {
    use crate::shared::domain::logging::LogRepository;

    pub struct FileLogRepository;

    impl LogRepository for FileLogRepository {
        fn log(&self, message: std::fmt::Arguments) -> Result<(), super::errors::InfrastructureError> {
            println!("Unhandable error: {}", message);
            Ok(())
        }
    }
}
