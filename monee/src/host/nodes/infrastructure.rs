pub mod repository {
    use monee_types::apps::{app_id::AppId, app_manifest::AppManifest};

    use crate::{host::nodes::domain::repository::Repository, shared::domain::context::DbContext};
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
                .query("SELECT id FROM ONLY node WHERE id = $id LIMIT 1")
                .bind(("id", id))
                .await?
                .check()?;

            let entity: Option<()> = response.take(0)?;
            Ok(entity.is_some())
        }
    }
}
