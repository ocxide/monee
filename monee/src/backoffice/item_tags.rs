pub mod application {
    pub mod create_one {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::{item_tag::ItemTag, repository::Repository},
            shared::{
                domain::{context::AppContext, errors::UniqueSaveStatus},
                infrastructure::errors::InfrastructureError,
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl CreateOne {
            pub async fn run(&self, tag: ItemTag) -> Result<UniqueSaveStatus, InfrastructureError> {
                let id = ItemTagId::new();
                self.repository.save(id, tag).await
            }
        }
    }

    pub mod relate {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::repository::{Repository, TagsRelation},
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
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
            ) -> Result<Status, InfrastructureError> {
                if parent_id == child_id {
                    return Ok(Status::CyclicRelation);
                }

                // Check if child_id is already parent of parent_id
                match self.repository.check_relation(parent_id, child_id).await? {
                    TagsRelation::TargetNotFound => return Ok(Status::NotFound(parent_id)),
                    TagsRelation::Ancestor => return Ok(Status::CyclicRelation),
                    TagsRelation::NotRelated => {}
                };

                match self.repository.link(parent_id, child_id).await? {
                    crate::shared::domain::errors::UniqueSaveStatus::Created => Ok(Status::Related),
                    crate::shared::domain::errors::UniqueSaveStatus::AlreadyExists => {
                        Ok(Status::AlreadyContains)
                    }
                }
            }
        }

        pub enum Status {
            Related,
            AlreadyContains,
            CyclicRelation,
            NotFound(monee_core::ItemTagId),
        }
    }

    pub mod unlink {
        use cream::context::ContextProvide;
        use monee_core::ItemTagId;

        use crate::{
            backoffice::item_tags::domain::repository::Repository,
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
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
            ) -> Result<(), InfrastructureError> {
                self.repository.unlink(parent_id, child_id).await
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_core::ItemTagId;

        use crate::shared::{
            domain::errors::UniqueSaveStatus, infrastructure::errors::InfrastructureError,
        };

        use super::item_tag::ItemTag;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                id: ItemTagId,
                tag: ItemTag,
            ) -> Result<UniqueSaveStatus, InfrastructureError>;

            async fn check_relation(
                &self,
                target_tag: ItemTagId,
                maybe_acestor: ItemTagId,
            ) -> Result<TagsRelation, InfrastructureError>;

            async fn link(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<UniqueSaveStatus, InfrastructureError>;

            async fn unlink(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), InfrastructureError>;
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
                domain::{
                    context::DbContext,
                    errors::{IntoDomainResult, UniqueSaveStatus},
                },
                infrastructure::{database::{Connection, EntityKey}, errors::InfrastructureError},
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(
                &self,
                id: ItemTagId,
                item_tag: ItemTag,
            ) -> Result<UniqueSaveStatus, InfrastructureError> {
                let response = self
                    .0
                    .query("CREATE type::thing('item_tag', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", item_tag))
                    .await?
                    .check();

                response.into_domain_result()
            }

            async fn check_relation(
                &self,
                target_tag: ItemTagId,
                maybe_ancestor: ItemTagId,
            ) -> Result<TagsRelation, InfrastructureError> {
                let mut response = self.0
                    .query(
                        "SELECT <-contains<-item_tag as parents FROM ONLY type::thing('item_tag', $parent_id)",
                    )
                    .bind(("parent_id", target_tag))
                    .await?
                    .check()?;

                let parents: Option<Vec<EntityKey<ItemTagId>>> = response.take("parents")?;

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
            ) -> Result<UniqueSaveStatus, InfrastructureError> {
                let response = self
                    .0
                    .query("LET $parent_thing = type::thing('item_tag', $parent_id)")
                    .bind(("parent_id", parent_id))
                    .query("LET $child_thing = type::thing('item_tag', $child_id)")
                    .bind(("child_id", child_id))
                    .query("RELATE $parent_thing->contains->$child_thing")
                    .await?
                    .check();

                response.into_domain_result()
            }

            async fn unlink(
                &self,
                parent_id: ItemTagId,
                child_id: ItemTagId,
            ) -> Result<(), InfrastructureError> {
                self.0.query("DELETE type::thing('item_tag', $parent_id)->contains WHERE out=type::thing('item_tag', $child_id)")
                    .bind(("parent_id", parent_id)).bind(("child_id", child_id))
                    .await?.check()?;

                Ok(())
            }
        }


        async fn check_multi_relation(
            connection: &crate::shared::infrastructure::database::Connection,
            parents: &[EntityKey<ItemTagId>],
            child_id: monee_core::ItemTagId,
        ) -> Result<TagsRelation, InfrastructureError> {
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

            let grand_parents: Vec<Vec<EntityKey<ItemTagId>>> = response.take("parents")?;
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
