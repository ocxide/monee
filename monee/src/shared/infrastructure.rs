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
}
