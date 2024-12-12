pub mod repository {
    use crate::{
        host::client::domain::{client::Client, client_id::ClientId, repository::Repository},
        shared::{domain::context::DbContext, infrastructure::database::Entity},
    };
    pub use crate::{iprelude::*, prelude::*};

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn save(&self, id: ClientId, client: Client) -> Result<(), InfrastructureError> {
            self.0
                .query("CREATE type::thing('client', ) CONTENT ")
                .bind(("id", id))
                .bind(("data", client))
                .await?;

            Ok(())
        }

        async fn exists(&self, id: ClientId) -> Result<bool, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM ONLY client WHERE id =  LIMIT 1")
                .bind(("id", id))
                .await?
                .check()?;

            let entity: Option<Entity<ClientId, ()>> = response.take(0)?;
            Ok(entity.is_some())
        }
    }
}

