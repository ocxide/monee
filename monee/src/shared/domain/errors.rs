use crate::prelude::AppError;

pub use monee_types::shared::errors::*;

impl From<surrealdb::Error> for AppError<UniqueSaveError> {
    fn from(value: surrealdb::Error) -> Self {
        use surrealdb::{error, Error};
        match value {
            Error::Api(error::Api::Query { .. }) => {
                AppError::App(UniqueSaveError::AlreadyExists("unknown"))
            }
            Error::Db(error::Db::IndexExists { thing, .. }) => {
                AppError::App(UniqueSaveError::AlreadyExists(match thing.tb.as_str() {
                    "event" => "event",
                    "wallet" => "wallet",
                    "currency" => "currency",
                    "actor" => "actor",
                    "item_tag" => "item_tag",
                    _ => "unknown",
                }))
            }
            e => AppError::Infrastructure(e.into()),
        }
    }
}
