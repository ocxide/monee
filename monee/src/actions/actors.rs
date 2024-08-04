pub mod list {
    pub type ActorRow =
        crate::shared::infrastructure::database::Entity<monee_core::actor::ActorId, monee_core::actor::Actor>;

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
    ) -> Result<Vec<ActorRow>, crate::shared::infrastructure::database::Error> {
        let mut response = connection.query("SELECT * FROM actor").await?.check()?;

        let actors: Vec<ActorRow> = response.take(0)?;
        Ok(actors)
    }
}

pub mod create {
    use monee_core::actor;

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Actor already exists")]
        AlreadyExists,
        #[error(transparent)]
        Database(#[from] crate::shared::infrastructure::database::Error),
    }

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        actor: actor::Actor,
    ) -> Result<actor::ActorId, Error> {
        let id = actor::ActorId::new();

        let result = connection
            .query("CREATE type::thing('actor', $id) CONTENT $data")
            .bind(("id", id))
            .bind(("data", actor))
            .await?
            .check();

        match result {
            Err(
                crate::shared::infrastructure::database::Error::Api(surrealdb::error::Api::Query { .. })
                | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
            ) => Err(Error::AlreadyExists),
            Err(e) => Err(e.into()),
            Ok(_) => Ok(id),
        }
    }
}

pub mod alias_get {
    use monee_core::actor;

    use crate::Entity;

    pub async fn run(
        connection: &crate::shared::infrastructure::database::Connection,
        alias: &str,
    ) -> Result<Option<actor::ActorId>, crate::shared::infrastructure::database::Error> {
        let mut response = connection
            .query("SELECT id FROM ONLY actor WHERE alias = $alias LIMIT 1")
            .bind(("alias", alias))
            .await?
            .check()?;

        let actor: Option<Entity<actor::ActorId, ()>> = response.take(0)?;
        Ok(actor.map(|a| a.0))
    }
}

