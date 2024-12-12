use crate::prelude::AppError;

pub use monee_types::shared::errors::*;

impl From<surrealdb::Error> for AppError<UniqueSaveError> {
    fn from(value: surrealdb::Error) -> Self {
        use surrealdb::{error, Error};
        match value {
            Error::Api(error::Api::Query { .. }) | Error::Db(error::Db::IndexExists { .. }) => {
                AppError::App(UniqueSaveError::AlreadyExists)
            }
            e => AppError::Infrastructure(e.into()),
        }
    }
}
