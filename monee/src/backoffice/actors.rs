pub mod domain {
    pub mod repository {
        use monee_core::actor::{Actor, ActorId};

        use crate::shared::infrastructure::errors::{UniqueSaveError, UnspecifiedError};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: ActorId, actor: Actor) -> Result<(), UniqueSaveError>;
            async fn alias_resolve(&self, name: &str) -> Result<Option<ActorId>, UnspecifiedError>;
        }
    }
}

pub mod application {
    pub mod create_one {
        use cream::from_context::FromContext;
        use monee_core::actor::{Actor, ActorId};

        use crate::{
            backoffice::actors::domain::repository::Repository,
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for CreateOne {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_actors_repository(),
                }
            }
        }

        impl CreateOne {
            pub async fn run(&self, actor: Actor) -> Result<(), Error> {
                self.repository
                    .save(ActorId::new(), actor)
                    .await
                    .map_err(|e| match e {
                        UniqueSaveError::AlreadyExists => Error::AlreadyExists,
                        UniqueSaveError::Unspecified(e) => Error::Unspecified(e),
                    })
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            Unspecified(#[from] UnspecifiedError),
            #[error("Actor already exists")]
            AlreadyExists,
        }
    }

    pub mod alias_resolve {
        use cream::from_context::FromContext;
        use monee_core::actor::ActorId;

        use crate::{
            backoffice::actors::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::UnspecifiedError},
        };

        pub struct AliasResolve {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for AliasResolve {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_actors_repository(),
                }
            }
        }

        impl AliasResolve {
            pub async fn run(&self, name: &str) -> Result<Option<ActorId>, UnspecifiedError> {
                self.repository.alias_resolve(name).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::actor::{Actor, ActorId};

        use crate::{
            backoffice::actors::domain::repository::Repository,
            shared::infrastructure::{
                database::{Connection, Entity},
                errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: ActorId, actor: Actor) -> Result<(), UniqueSaveError> {
                let result = self
                    .0
                    .query("CREATE type::thing('actor', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", actor))
                    .await
                    .map_err(UnspecifiedError::from)?
                    .check();

                match result {
                    Ok(_) => Ok(()),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(UniqueSaveError::AlreadyExists),
                    Err(e) => Err(UniqueSaveError::Unspecified(e.into())),
                }
            }

            async fn alias_resolve(
                &self,
                alias: &str,
            ) -> Result<Option<ActorId>, UnspecifiedError> {
                let mut response = self
                    .0
                    .query("SELECT id FROM ONLY actor WHERE alias = $alias LIMIT 1")
                    .bind(("alias", alias))
                    .await?
                    .check()?;

                let actor: Option<Entity<ActorId, ()>> = response.take(0)?;
                Ok(actor.map(|a| a.0))
            }
        }
    }
}
