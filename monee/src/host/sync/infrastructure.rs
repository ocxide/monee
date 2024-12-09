pub mod repository {
    use crate::{
        host::sync::domain::{repository::Repository, sync_guide::SyncGuide},
        iprelude::*,
        prelude::*,
        shared::domain::{context::DbContext, date::Datetime},
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn get_sync_guide(&self) -> Result<SyncGuide, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT created_at FROM events ORDER BY created_at DESC LIMIT 1")
                .await?;
            let last_event_date: Option<Datetime> = response.take(0)?;

            Ok(SyncGuide { last_event_date })
        }
    }
}
