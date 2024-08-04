pub mod domain {
    pub mod repository {
        use monee_core::actor::{Actor, ActorId};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: ActorId, actor: Actor) -> Result<(), SaveError>;
            async fn alias_resolve(
                &self,
                name: &str,
            ) -> Result<Option<ActorId>, crate::shared::errors::InfrastructureError>;
        }

        #[derive(thiserror::Error, Debug)]
        pub enum SaveError {
            #[error(transparent)]
            Infrastructure(#[from] crate::shared::infrastructure::database::Error),
            #[error("Actor already exists")]
            AlreadyExists,
        }
    }
}

pub mod application {
    pub mod create_one {
        use cream::from_context::FromContext;
        use monee_core::actor::{Actor, ActorId};

        use crate::{
            backoffice::actors::domain::repository::{Repository, SaveError},
            shared::domain::context::AppContext,
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
            pub async fn run(&self, actor: Actor) -> Result<(), SaveError> {
                self.repository.save(ActorId::new(), actor).await
            }
        }
    }

    pub mod alias_resolve {
        use cream::from_context::FromContext;
        use monee_core::actor::ActorId;

        use crate::{
            backoffice::actors::domain::repository::Repository, shared::domain::context::AppContext,
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
            pub async fn run(
                &self,
                name: &str,
            ) -> Result<Option<ActorId>, crate::shared::errors::InfrastructureError> {
                self.repository.alias_resolve(name).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::actor::{Actor, ActorId};

        use crate::{
            backoffice::actors::domain::repository::{Repository, SaveError},
            shared::infrastructure::database::{Connection, Entity},
        };

        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: ActorId, actor: Actor) -> Result<(), SaveError> {
                let result = self
                    .0
                    .query("CREATE type::thing('actor', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", actor))
                    .await
                    .map_err(SaveError::Infrastructure)?
                    .check();

                match result {
                    Ok(_) => Ok(()),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(SaveError::AlreadyExists),
                    Err(e) => Err(SaveError::Infrastructure(e)),
                }
            }

            async fn alias_resolve(
                &self,
                alias: &str,
            ) -> Result<Option<ActorId>, crate::shared::errors::InfrastructureError> {
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
