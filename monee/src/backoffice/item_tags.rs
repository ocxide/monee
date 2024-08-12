pub mod application {
    pub mod create_one {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::{item_tag::ItemTag, repository::Repository},
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl CreateOne {
            pub async fn run(&self, tag: ItemTag) -> Result<(), Error> {
                let id = ItemTagId::new();
                self.repository.save(id, tag).await.map_err(Error::from)
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error("Item tag already exists")]
            AlreadyExists,
            #[error(transparent)]
            Unspecified(#[from] UnspecifiedError),
        }

        impl From<UniqueSaveError> for Error {
            fn from(err: UniqueSaveError) -> Self {
                match err {
                    UniqueSaveError::AlreadyExists => Self::AlreadyExists,
                    UniqueSaveError::Unspecified(err) => Self::Unspecified(err),
                }
            }
        }
    }

    pub mod relate {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::repository::{Repository, TagsRelation},
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct Relate {
            repository: Box<dyn Repository>,
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

                // Check if child_id is already parent of parent_id
                match self.repository.check_relation(parent_id, child_id).await? {
                    TagsRelation::TargetNotFound => return Err(Error::NotFound(parent_id)),
                    TagsRelation::Ancestor => return Err(Error::CyclicRelation),
                    TagsRelation::NotRelated => {}
                };

                self.repository
                    .link(parent_id, child_id)
                    .await
                    .map_err(|e| match e {
                        UniqueSaveError::Unspecified(e) => Error::Unspecified(e),
                        UniqueSaveError::AlreadyExists => Error::AlreadyContains,
                    })?;

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
            NotFound(monee_core::ItemTagId),
            #[error(transparent)]
            Unspecified(#[from] UnspecifiedError),
        }
    }

    pub mod unlink {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::UnspecifiedError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct Unlink {
            repository: Box<dyn Repository>,
        }

        impl Unlink {
            pub async fn run(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), UnspecifiedError> {
                self.repository.unlink(parent_id, child_id).await
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_core::ItemTagId;

        use crate::shared::infrastructure::errors::{UniqueSaveError, UnspecifiedError};

        use super::item_tag::ItemTag;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: ItemTagId, tag: ItemTag) -> Result<(), UniqueSaveError>;

            async fn check_relation(
                &self,
                target_tag: ItemTagId,
                maybe_acestor: ItemTagId,
            ) -> Result<TagsRelation, UnspecifiedError>;

            async fn link(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), UniqueSaveError>;

            async fn unlink(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), UnspecifiedError>;
        }

        pub enum TagsRelation {
            Ancestor,
            NotRelated,
            TargetNotFound,
        }
    }

    pub mod item_tag {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct ItemTag {
            pub name: String,
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::{
                item_tag::ItemTag,
                repository::{Repository, TagsRelation},
            },
            shared::{
                domain::context::DbContext,
                infrastructure::{
                    database::Connection,
                    errors::{UniqueSaveError, UnspecifiedError},
                },
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: ItemTagId, item_tag: ItemTag) -> Result<(), UniqueSaveError> {
                self.0
                    .query("CREATE type::thing('item_tag', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", item_tag))
                    .await
                    .map_err(UnspecifiedError::from)?
                    .check()
                    .map_err(|e| match e {
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | crate::shared::infrastructure::database::Error::Db(
                            surrealdb::error::Db::IndexExists { .. },
                        ) => UniqueSaveError::AlreadyExists,
                        e => UniqueSaveError::Unspecified(e.into()),
                    })?;

                Ok(())
            }

            async fn check_relation(
                &self,
                target_tag: ItemTagId,
                maybe_ancestor: ItemTagId,
            ) -> Result<TagsRelation, UnspecifiedError> {
                let mut response = self.0
                    .query(
                        "SELECT <-contains<-item_tag as parents FROM ONLY type::thing('item_tag', $parent_id)",
                    )
                    .bind(("parent_id", target_tag))
                    .await?
                    .check()?;

                let parents: Option<Vec<ParentTagId>> = response.take("parents")?;

                let parents = match parents.as_deref() {
                    Some([]) => return Ok(TagsRelation::NotRelated),
                    Some(parents) => parents,
                    None => return Ok(TagsRelation::TargetNotFound),
                };

                if parents.iter().any(|p| p.0 == maybe_ancestor) {
                    return Ok(TagsRelation::Ancestor);
                }

                let relation = check_multi_relation(&self.0, parents, maybe_ancestor).await?;
                Ok(relation)
            }

            async fn link(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), UniqueSaveError> {
                let response = self
                    .0
                    .query("LET $parent_thing = type::thing('item_tag', $parent_id)")
                    .bind(("parent_id", parent_id))
                    .query("LET $child_thing = type::thing('item_tag', $child_id)")
                    .bind(("child_id", child_id))
                    .query("RELATE $parent_thing->contains->$child_thing")
                    .await
                    .map_err(UnspecifiedError::from)?
                    .check();

                match response {
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

            async fn unlink(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), UnspecifiedError> {
                self.0.query("DELETE type::thing('item_tag', $parent_id)->contains WHERE out=type::thing('item_tag', $child_id)")
                    .bind(("parent_id", parent_id)).bind(("child_id", child_id))
                    .await?.check()?;

                Ok(())
            }
        }

        #[derive(serde::Deserialize, Debug)]
        struct ParentTagId(#[serde(with = "crate::sql_id::string")] monee_core::ItemTagId);

        async fn check_multi_relation(
            connection: &crate::shared::infrastructure::database::Connection,
            parents: &[ParentTagId],
            child_id: monee_core::ItemTagId,
        ) -> Result<TagsRelation, UnspecifiedError> {
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
                return Ok(TagsRelation::NotRelated);
            }

            if grand_parents.iter().any(|p| p.0 == child_id) {
                return Ok(TagsRelation::Ancestor);
            }

            Box::pin(check_multi_relation(connection, &grand_parents, child_id)).await
        }
    }
}
