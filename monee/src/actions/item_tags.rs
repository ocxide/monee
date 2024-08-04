pub mod view {
    use monee_core::item_tag;

    pub struct ItemTagMini {
        pub name: String,
    }

    #[derive(serde::Deserialize)]
    struct ItemTagSelect {
        #[serde(flatten)]
        item_tag: item_tag::ItemTag,
        children: Vec<String>,
    }

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
    ) -> Result<Vec<(item_tag::ItemTag, Vec<String>)>, crate::shared::infrastructure::database::Error> {
        let mut response = connection
            .query("SELECT ->contains->item_tag.name as children, name FROM item_tag")
            .await?
            .check()?;

        let tags: Vec<ItemTagSelect> = response.take(0)?;
        Ok(tags.into_iter().map(|t| (t.item_tag, t.children)).collect())
    }
}

pub mod create {
    use monee_core::item_tag::ItemTag;

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Item tag already exists")]
        AlreadyExists,
        #[error(transparent)]
        Database(#[from] crate::shared::infrastructure::database::Error),
    }

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        item_tag: ItemTag,
    ) -> Result<(), Error> {
        let id = monee_core::item_tag::ItemTagId::new();
        connection
            .query("CREATE type::thing('item_tag', $id) CONTENT $data")
            .bind(("id", id))
            .bind(("data", item_tag))
            .await?
            .check()
            .map_err(|e| match e {
                crate::shared::infrastructure::database::Error::Api(surrealdb::error::Api::Query { .. })
                | crate::shared::infrastructure::database::Error::Db(surrealdb::error::Db::IndexExists { .. }) => {
                    Error::AlreadyExists
                }
                e => Error::Database(e),
            })?;

        Ok(())
    }
}

pub mod get {
    use crate::Entity;

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        name: &str,
    ) -> Result<Option<monee_core::item_tag::ItemTagId>, crate::shared::infrastructure::database::Error> {
        let mut response = connection
            .query("SELECT id FROM ONLY item_tag WHERE name = $name LIMIT 1")
            .bind(("name", name))
            .await?
            .check()?;

        let tag: Option<Entity<monee_core::item_tag::ItemTagId, ()>> = response.take(0)?;
        Ok(tag.map(|t| t.0))
    }
}

pub mod relate {
    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Item tag already contains item tag")]
        AlreadyContains,
        #[error("Cyclic relation")]
        CyclicRelation,
        #[error("Item tag `{0}` not found")]
        NotFound(monee_core::item_tag::ItemTagId),
        #[error(transparent)]
        Database(#[from] crate::shared::infrastructure::database::Error),
    }

    #[derive(serde::Deserialize, Debug)]
    struct ParentTagId(#[serde(with = "crate::sql_id::string")] monee_core::item_tag::ItemTagId);

    async fn check_multi_relation(
        connection: &crate::shared::infrastructure::database::Connection,
        parents: &[ParentTagId],
        child_id: monee_core::item_tag::ItemTagId,
    ) -> Result<(), Error> {
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
            return Ok(());
        }

        if grand_parents.iter().any(|p| p.0 == child_id) {
            return Err(Error::CyclicRelation);
        }

        Box::pin(check_multi_relation(connection, &grand_parents, child_id)).await
    }

    async fn check_relation(
        connection: &crate::shared::infrastructure::database::Connection,
        parent_id: monee_core::item_tag::ItemTagId,
        child_id: monee_core::item_tag::ItemTagId,
    ) -> Result<(), Error> {
        let mut response = connection
            .query(
                "SELECT <-contains<-item_tag as parents FROM ONLY type::thing('item_tag', $parent_id)",
            )
            .bind(("parent_id", parent_id))
            .await?
            .check()?;

        let parents: Option<Vec<ParentTagId>> = response.take("parents")?;

        let parents = match parents.as_deref() {
            Some([]) => return Ok(()),
            Some(parents) => parents,
            None => return Err(Error::NotFound(parent_id)),
        };

        if parents.iter().any(|p| p.0 == child_id) {
            return Err(Error::CyclicRelation);
        }

        check_multi_relation(connection, parents, child_id).await
    }

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        parent_id: monee_core::item_tag::ItemTagId,
        child_id: monee_core::item_tag::ItemTagId,
    ) -> Result<(), Error> {
        if parent_id == child_id {
            return Err(Error::CyclicRelation);
        }

        check_relation(connection, parent_id, child_id).await?;

        let response = connection
            .query("LET $parent_thing = type::thing('item_tag', $parent_id)")
            .bind(("parent_id", parent_id))
            .query("LET $child_thing = type::thing('item_tag', $child_id)")
            .bind(("child_id", child_id))
            .query("RELATE $parent_thing->contains->$child_thing")
            .await?
            .check();

        match response {
            Ok(_) => Ok(()),
            Err(
                crate::shared::infrastructure::database::Error::Api(surrealdb::error::Api::Query { .. })
                | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
            ) => Err(Error::AlreadyContains),
            Err(e) => Err(Error::Database(e)),
        }
    }
}

pub mod unlink {
    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Item tag `{0}` not found")]
        NotFound(monee_core::item_tag::ItemTagId),
        #[error(transparent)]
        Database(#[from] crate::shared::infrastructure::database::Error),
    }

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        parent_id: monee_core::item_tag::ItemTagId,
        child_id: monee_core::item_tag::ItemTagId,
    ) -> Result<(), Error> {
        connection.query("DELETE type::thing('item_tag', $parent_id)->contains WHERE out=type::thing('item_tag', $child_id)")
            .bind(("parent_id", parent_id)).bind(("child_id", child_id))
            .await?.check()?;

        Ok(())
    }
}
