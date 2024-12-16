pub mod repository {
    use cream::context::FromContext;
    use monee_core::ItemTagId;

    use crate::{
        backoffice::item_tags::domain::{
            item_name::ItemName,
            item_tag::ItemTag,
            item_tag_node::ItemTagNode,
            repository::{Repository, TagsRelation},
        },
        iprelude::*,
        prelude::*,
        shared::{
            domain::{context::DbContext, errors::UniqueSaveError},
            infrastructure::database::{Connection, EntityKey},
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn save(
            &self,
            id: ItemTagId,
            item_tag: ItemTag,
        ) -> Result<(), AppError<UniqueSaveError>> {
            let response = self
                .0
                .query("CREATE type::thing('item_tag', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", item_tag))
                .await
                .catch_infra()?
                .check();

            response.catch_app().map_response()
        }

        async fn check_relation(
            &self,
            target_tag: ItemTagId,
            maybe_ancestor: ItemTagId,
        ) -> Result<TagsRelation, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT <-contains<-item_tag as parents FROM ONLY type::thing('item_tag', $parent_id)")
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
        ) -> Result<(), AppError<UniqueSaveError>> {
            let response = self
                .0
                .query("LET $eparent_id = type::thing('item_tag', $parent_id)")
                .bind(("parent_id", parent_id))
                .query("LET $echild_id = type::thing('item_tag', $child_id)")
                .bind(("child_id", child_id))
                .query("RELATE $eparent_id->contains->$echild_id")
                .await
                .catch_infra()?
                .check();

            response.catch_app().map_response()
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

        async fn name_resolve(
            &self,
            name: ItemName,
        ) -> Result<Option<ItemTagId>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM item_tag WHERE name = $name")
                .bind(("name", name))
                .await?
                .check()?;

            let id: Option<EntityKey<monee_core::ItemTagId>> = response.take("id")?;
            Ok(id.map(|k| k.0))
        }

        async fn get_all(&self) -> Result<Vec<ItemTagNode>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT *, <-contains<-item_tag.name as parents_name FROM item_tag")
                .await?
                .check()?;

            #[derive(serde::Deserialize)]
            struct SqlItemTagNode {
                id: EntityKey<ItemTagId>,
                #[serde(flatten)]
                tag: ItemTag,
                parents_name: Vec<ItemName>,
            }

            impl From<SqlItemTagNode> for ItemTagNode {
                fn from(node: SqlItemTagNode) -> Self {
                    Self {
                        id: node.id.0,
                        tag: node.tag,
                        parents_name: node.parents_name,
                    }
                }
            }

            let nodes: Vec<SqlItemTagNode> = response.take(0)?;
            Ok(nodes.into_iter().map(Into::into).collect())
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
            .query("SELECT <-contains<-item_tag as parents FROM ")
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
