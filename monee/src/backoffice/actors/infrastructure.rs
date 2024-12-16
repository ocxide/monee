pub mod repository {
    use cream::context::FromContext;
    use monee_core::ActorId;

    use crate::{
        backoffice::actors::domain::{
            actor::Actor, actor_alias::ActorAlias, repository::Repository,
        },
        iprelude::*,
        prelude::*,
        shared::{
            domain::{context::DbContext, errors::UniqueSaveError},
            infrastructure::database::{Connection, Entity},
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn save(&self, id: ActorId, actor: Actor) -> Result<(), AppError<UniqueSaveError>> {
            let result = self
                .0
                .query("CREATE type::thing('actor', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", actor))
                .await
                .catch_infra()?
                .check();

            result.catch_app().map_response()
        }

        async fn alias_resolve(
            &self,
            alias: ActorAlias,
        ) -> Result<Option<ActorId>, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM ONLY actor WHERE alias = $alias LIMIT 1")
                .bind(("alias", alias))
                .await?
                .check()?;

            let actor: Option<Entity<ActorId, ()>> = response.take(0)?;
            Ok(actor.map(Entity::into_key))
        }

        async fn get_all(&self) -> Result<Vec<(ActorId, Actor)>, InfrastructureError> {
            let mut response = self.0.query("SELECT * FROM actor").await?.check()?;

            let actors: Vec<Entity<ActorId, Actor>> = response.take(0)?;
            Ok(actors.into_iter().map(From::from).collect())
        }
    }
}
