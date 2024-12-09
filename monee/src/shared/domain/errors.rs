use crate::shared::infrastructure::errors::IntoAppResult;

pub enum UniqueSaveError {
    AlreadyExists,
}

impl IntoAppResult<UniqueSaveError> for Result<surrealdb::Response, surrealdb::Error> {
    fn into_app_result(
        self,
    ) -> Result<(), crate::shared::infrastructure::errors::AppError<UniqueSaveError>> {
        match self {
            Ok(_) => Ok(()),
            Err(
                crate::shared::infrastructure::database::Error::Api(surrealdb::error::Api::Query {
                    ..
                })
                | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
            ) => Err(crate::shared::infrastructure::errors::AppError::App(
                UniqueSaveError::AlreadyExists,
            )),
            Err(e) => {
                Err(crate::shared::infrastructure::errors::AppError::Infrastructure(e.into()))
            }
        }
    }
}
