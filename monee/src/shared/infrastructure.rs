pub mod database;

pub mod errors {
    #[derive(Debug, thiserror::Error)]
    #[error("infrastructure error: {0}")]
    pub struct UnspecifiedError(Box<dyn std::error::Error + Send + Sync + 'static>);

    impl UnspecifiedError {
        pub fn new<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
            Self(Box::new(err))
        }
    }

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

    #[derive(Debug)]
    pub enum AppError<E> {
        App(E),
        Infrastructure(InfrastructureError),
    }

    impl<E> From<InfrastructureError> for AppError<E> {
        fn from(value: InfrastructureError) -> Self {
            Self::Infrastructure(value)
        }
    }
}

pub mod logging {
    use crate::shared::domain::logging::LogRepository;

    pub struct FileLogRepository;

    impl LogRepository for FileLogRepository {
        fn log(
            &self,
            message: std::fmt::Arguments,
        ) -> Result<(), super::errors::InfrastructureError> {
            println!("Unhandable error: {}", message);
            Ok(())
        }
    }
}

pub mod filesystem {
    #[cfg(feature = "embedded")]
    pub fn create_local_path() -> std::path::PathBuf {
        use std::{fs, path::PathBuf};

        let share_dir = std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".local/share"))
            })
            .expect("To get share directory");
        let path = share_dir.join("monee");

        fs::create_dir_all(&path).expect("To create monee data directory");
        path
    }
}
