pub mod domain {
    pub mod repository {
        use cream::context::FromContext;
        use monee_core::ActorId;

        use crate::shared::{
            domain::context::AppContext,
            infrastructure::errors::{UniqueSaveError, UnspecifiedError},
        };

        use super::actor::Actor;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: ActorId, actor: Actor) -> Result<(), UniqueSaveError>;
            async fn alias_resolve(&self, name: &str) -> Result<Option<ActorId>, UnspecifiedError>;
        }

        impl<C: AppContext> FromContext<C> for Box<dyn Repository> {
            fn from_context(context: &C) -> Self {
                context.backoffice_actors_repository()
            }
        }
    }

    pub mod actor {
        use super::actor_type::ActorType;

        #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
        pub struct Actor {
            pub name: String,
            #[serde(rename = "type")]
            pub actor_type: ActorType,
            pub alias: Option<String>,
        }
    }

    pub mod actor_type {
        #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
        pub enum ActorType {
            Natural,
            Business,
            FinancialEntity,
        }

        pub mod actor_type_from_str {
            use std::str::FromStr;

            use super::ActorType;

            #[derive(Debug)]
            pub struct Error {}

            impl std::fmt::Display for Error {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(
                        f,
                        "invalid actor type, must be 'natural', 'business', or 'financial_entity'"
                    )
                }
            }

            impl std::error::Error for Error {}

            impl FromStr for ActorType {
                type Err = Error;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        "natural" | "n" => Ok(Self::Natural),
                        "business" | "b" => Ok(Self::Business),
                        "financial_entity" | "f" => Ok(Self::FinancialEntity),
                        _ => Err(Error {}),
                    }
                }
            }
        }
    }
}

pub mod application {
    pub mod create_one {
        use cream::context::FromContext;
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::{actor::Actor, repository::Repository},
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        #[derive(FromContext)]
        #[from_context(C: AppContext)]
        pub struct CreateOne {
            repository: Box<dyn Repository>,
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
        use cream::context::FromContext;
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::UnspecifiedError},
        };

        #[derive(FromContext)]
        #[from_context(C: AppContext)]
        pub struct AliasResolve {
            repository: Box<dyn Repository>,
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
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::{actor::Actor, repository::Repository},
            shared::infrastructure::{
                database::{Connection, Entity},
                errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        pub struct SurrealRepository(Connection);
        impl SurrealRepository {
            pub(crate) fn new(
                clone: surrealdb::Surreal<surrealdb::engine::remote::ws::Client>,
            ) -> Self {
                Self(clone)
            }
        }

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
