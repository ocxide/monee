use crate::shared::infrastructure::errors::IntoAppResult;

#[derive(PartialEq, Eq)]
pub enum UniqueSaveStatus {
    Created,
    AlreadyExists,
}

impl UniqueSaveStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, UniqueSaveStatus::Created)
    }
}

pub enum UniqueUpdateStatus {
    Updated,
    NotFound,
    Conflict,
}

pub enum UniqueSaveError {
    AlreadyExists,
}

pub(crate) trait IntoDomainResult<T, E> {
    fn into_domain_result(self) -> Result<T, E>;
}

impl
    IntoDomainResult<
        UniqueSaveStatus,
        crate::shared::infrastructure::errors::InfrastructureError,
    > for Result<surrealdb::Response, surrealdb::Error>
{
    fn into_domain_result(
        self,
    ) -> Result<UniqueSaveStatus, crate::shared::infrastructure::errors::InfrastructureError>
    {
        match self {
            Ok(_) => Ok(UniqueSaveStatus::Created),
            Err(
                crate::shared::infrastructure::database::Error::Api(
                    surrealdb::error::Api::Query { .. },
                )
                | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
            ) => Ok(UniqueSaveStatus::AlreadyExists),
            Err(e) => Err(e.into()),
        }
    }
}

impl IntoAppResult<UniqueSaveError> for Result<surrealdb::Response, surrealdb::Error> {
    fn into_app_result(
        self,
    ) -> Result<(), crate::shared::infrastructure::errors::AppError<UniqueSaveError>> {
        match self {
            Ok(_) => Ok(()),
            Err(
                crate::shared::infrastructure::database::Error::Api(
                    surrealdb::error::Api::Query { .. },
                )
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

