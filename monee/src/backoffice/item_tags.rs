pub mod application {
    pub mod create_one {
        use cream::from_context::FromContext;
        use monee_core::item_tag::{ItemTag, ItemTagId};

        use crate::{
            backoffice::item_tags::domain::repository::{Repository, SaveError},
            shared::domain::context::AppContext,
        };

        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for CreateOne {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_item_tags_repository(),
                }
            }
        }

        impl CreateOne {
            pub async fn run(&self, tag: ItemTag) -> Result<(), SaveError> {
                let id = ItemTagId::new();
                self.repository.save(id, tag).await
            }
        }
    }

    pub mod relate {
        use cream::from_context::FromContext;
        use monee_core::item_tag::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::repository::Repository,
            shared::{domain::context::AppContext, errors::InfrastructureError},
        };

        pub struct Relate {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for Relate {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_item_tags_repository(),
                }
            }
        }

        impl Relate {
            pub async fn run(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), Error> {
                if parent_id == child_id {
                    return Err(Error::CyclicRelation);
                }

                match self.repository.check_relation(parent_id, child_id).await? {
                    None => return Err(Error::NotFound(parent_id)),
                    Some(true) => return Err(Error::CyclicRelation),
                    _ => {}
                };

                let linked = self.repository.link(parent_id, child_id).await?;
                if !linked {
                    return Err(Error::AlreadyContains);
                }

                Ok(())
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error("Item tag already contains item tag")]
            AlreadyContains,
            #[error("Cyclic relation")]
            CyclicRelation,
            #[error("Item tag `{0}` not found")]
            NotFound(monee_core::item_tag::ItemTagId),
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_core::item_tag::{ItemTag, ItemTagId};

        use crate::shared::errors::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: ItemTagId, tag: ItemTag) -> Result<(), SaveError>;
            async fn check_relation(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<Option<bool>, InfrastructureError>;
            async fn link(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<bool, InfrastructureError>;
        }

        #[derive(thiserror::Error, Debug)]
        pub enum SaveError {
            #[error("Item tag already exists")]
            AlreadyExists,
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::item_tag::{ItemTag, ItemTagId};

        use crate::{
            backoffice::item_tags::domain::repository::{Repository, SaveError},
            shared::{errors::InfrastructureError, infrastructure::database::Connection},
        };

        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: ItemTagId, item_tag: ItemTag) -> Result<(), SaveError> {
                self.0
                    .query("CREATE type::thing('item_tag', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", item_tag))
                    .await
                    .map_err(InfrastructureError::new)?
                    .check()
                    .map_err(|e| match e {
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | crate::shared::infrastructure::database::Error::Db(
                            surrealdb::error::Db::IndexExists { .. },
                        ) => SaveError::AlreadyExists,
                        e => SaveError::Infrastructure(e.into()),
                    })?;

                Ok(())
            }

            async fn check_relation(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<Option<bool>, InfrastructureError> {
                let mut response = self.0
                    .query(
                        "SELECT <-contains<-item_tag as parents FROM ONLY type::thing('item_tag', $parent_id)",
                    )
                    .bind(("parent_id", parent_id))
                    .await?
                    .check()?;

                let parents: Option<Vec<ParentTagId>> = response.take("parents")?;

                let parents = match parents.as_deref() {
                    Some([]) => return Ok(Some(false)),
                    Some(parents) => parents,
                    None => return Ok(None),
                };

                if parents.iter().any(|p| p.0 == child_id) {
                    return Ok(Some(true));
                }

                let has_relation = check_multi_relation(&self.0, parents, child_id).await?;
                Ok(Some(has_relation))
            }

            async fn link(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<bool, InfrastructureError> {
                let response = self
                    .0
                    .query("LET $parent_thing = type::thing('item_tag', $parent_id)")
                    .bind(("parent_id", parent_id))
                    .query("LET $child_thing = type::thing('item_tag', $child_id)")
                    .bind(("child_id", child_id))
                    .query("RELATE $parent_thing->contains->$child_thing")
                    .await?
                    .check();

                match response {
                    Ok(_) => Ok(true),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Ok(false),
                    Err(e) => Err(InfrastructureError::new(e)),
                }
            }
        }

        #[derive(serde::Deserialize, Debug)]
        struct ParentTagId(
            #[serde(with = "crate::sql_id::string")] monee_core::item_tag::ItemTagId,
        );

        async fn check_multi_relation(
            connection: &crate::shared::infrastructure::database::Connection,
            parents: &[ParentTagId],
            child_id: monee_core::item_tag::ItemTagId,
        ) -> Result<bool, InfrastructureError> {
            let parents = parents
                .iter()
                .map(|p| {
                    let id = surrealdb::sql::Id::String(p.0.to_string());
                    surrealdb::sql::Thing::from(("item_tag", id))
                })
                .collect::<Vec<_>>();

            let mut response = connection
                .query("SELECT <-contains<-item_tag as parents FROM $items")
                .bind(("items", parents))
                .await?
                .check()?;

            let grand_parents: Vec<Vec<ParentTagId>> = response.take("parents")?;
            let grand_parents: Vec<_> = grand_parents
                .into_iter()
                .filter(|p| !p.is_empty())
                .flat_map(|p| p.into_iter())
                .collect();

            if grand_parents.is_empty() {
                return Ok(false);
            }

            if grand_parents.iter().any(|p| p.0 == child_id) {
                return Ok(true);
            }

            Box::pin(check_multi_relation(connection, &grand_parents, child_id)).await
        }
    }
}
