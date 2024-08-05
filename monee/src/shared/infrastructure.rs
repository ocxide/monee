pub mod database;

pub mod errors {
    #[derive(Debug, thiserror::Error)]
    #[error("infrastructure error: {0}")]
    pub struct UnspecifiedError(Box<dyn std::error::Error>);

    impl UnspecifiedError {
        pub fn new<E>(err: E) -> Self
        where
            E: Into<Box<dyn std::error::Error>>,
        {
            Self(err.into())
        }
    }

    impl From<surrealdb::Error> for UnspecifiedError {
        fn from(err: surrealdb::Error) -> Self {
            Self(err.into())
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum UniqueSaveError {
        #[error("Item already exists")]
        AlreadyExists,
        #[error(transparent)]
        Unspecified(#[from] UnspecifiedError),
    }
}
