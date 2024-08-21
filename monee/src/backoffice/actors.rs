pub mod domain {
    pub mod repository {
        use monee_core::ActorId;

        use crate::{
            prelude::AppError,
            shared::{
                domain::errors::UniqueSaveError, infrastructure::errors::InfrastructureError,
            },
        };

        use super::{actor::Actor, actor_alias::ActorAlias};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                id: ActorId,
                actor: Actor,
            ) -> Result<(), AppError<UniqueSaveError>>;
            async fn alias_resolve(
                &self,
                name: &ActorAlias,
            ) -> Result<Option<ActorId>, InfrastructureError>;
        }
    }

    pub mod actor {
        use super::{actor_alias::ActorAlias, actor_name::ActorName, actor_type::ActorType};

        #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
        pub struct Actor {
            pub name: ActorName,
            #[serde(rename = "type")]
            pub actor_type: ActorType,
            pub alias: Option<ActorAlias>,
        }
    }

    pub mod actor_name {
        #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
        pub struct ActorName(String);

        impl From<String> for ActorName {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    }

    pub mod actor_alias {
        use std::{fmt::Display, str::FromStr};

        use crate::shared::domain::alias::{from_str::Error, Alias};

        #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
        pub struct ActorAlias(Alias);

        impl Display for ActorAlias {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for ActorAlias {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Alias::from_str(s)?))
            }
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
        use cream::context::ContextProvide;
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::{actor::Actor, repository::Repository},
            prelude::AppError,
            shared::domain::{context::AppContext, errors::UniqueSaveError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl CreateOne {
            pub async fn run(&self, actor: Actor) -> Result<(), AppError<UniqueSaveError>> {
                self.repository.save(ActorId::new(), actor).await
            }
        }
    }

    pub mod alias_resolve {
        use cream::context::ContextProvide;
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::{actor_alias::ActorAlias, repository::Repository},
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct AliasResolve {
            repository: Box<dyn Repository>,
        }

        impl AliasResolve {
            pub async fn run(
                &self,
                name: &ActorAlias,
            ) -> Result<Option<ActorId>, InfrastructureError> {
                self.repository.alias_resolve(name).await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::ContextProvide;
        use monee_core::ActorId;

        use crate::{
            backoffice::actors::domain::{
                actor::Actor, actor_alias::ActorAlias, repository::Repository,
            },
            prelude::AppError,
            shared::{
                domain::{context::DbContext, errors::UniqueSaveError},
                infrastructure::{
                    database::{Connection, Entity},
                    errors::{InfrastructureError, IntoAppResult},
                },
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(
                &self,
                id: ActorId,
                actor: Actor,
            ) -> Result<(), AppError<UniqueSaveError>> {
                let result = self
                    .0
                    .query("CREATE type::thing('actor', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", actor))
                    .await
                    .map_err(InfrastructureError::from)?
                    .check();

                result.into_app_result()
            }

            async fn alias_resolve(
                &self,
                alias: &ActorAlias,
            ) -> Result<Option<ActorId>, InfrastructureError> {
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
