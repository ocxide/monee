pub mod repository {
    use monee_types::apps::{app_id::AppId, app_manifest::AppManifest};

    use crate::{
        host::nodes::domain::repository::Repository,
        shared::{domain::context::DbContext, infrastructure::database::EntityKey},
    };
    pub use crate::{iprelude::*, prelude::*};

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn save(&self, id: AppId, app: AppManifest) -> Result<(), InfrastructureError> {
            self.0
                .query("CREATE type::thing('node', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", app))
                .await?;

            Ok(())
        }

        async fn exists(&self, id: AppId) -> Result<bool, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT id FROM ONLY type::thing('node', $id) LIMIT 1")
                .bind(("id", id))
                .await?
                .check()?;

            let entity: Option<EntityKey<AppId>> = response.take("id")?;
            Ok(entity.is_some())
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(unused)]
        use crate::shared::infrastructure::database::connect;

        use super::*;

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn checks_exists() {
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let repo: SurrealRepository = ctx.provide();
            let id = AppId::default();
            repo.save(id, AppManifest { name: None })
                .await
                .expect("To save");

            assert!(repo.exists(id).await.expect("To check"));
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn fails_doesnt_exist() {
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let repo: SurrealRepository = ctx.provide();
            let id = AppId::default();

            assert!(!repo.exists(id).await.expect("To check"));
        }
    }
}
