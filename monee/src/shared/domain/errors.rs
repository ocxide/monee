use crate::prelude::AppError;

pub use monee_types::shared::errors::*;

impl From<surrealdb::Error> for AppError<UniqueSaveError> {
    fn from(value: surrealdb::Error) -> Self {
        use surrealdb::{error, Error};
        match value {
            Error::Api(error::Api::Query { .. }) => {
                AppError::App(UniqueSaveError::AlreadyExists("unknown".to_owned()))
            }
            Error::Db(error::Db::IndexExists { thing, .. }) => {
                AppError::App(UniqueSaveError::AlreadyExists(thing.tb))
            }
            e => AppError::Infrastructure(e.into()),
        }
    }
}
