pub mod repository {
    use crate::{
        host::{
            client::domain::client_id::ClientId,
            sync::domain::{
                repository::Repository, sync_data::SyncData, sync_error::SyncError,
                sync_guide::SyncGuide,
            },
        },
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

        async fn save_sync(
            &self,
            client_id: ClientId,
            sync: &SyncData,
        ) -> Result<(), InfrastructureError> {
            self.0
                .query("UPSERT type::thing('client_sync', $client_id) REPLACE { data: $data }")
                .bind(("client_id", client_id))
                .bind(("data", serde_json::to_string(sync).unwrap()))
                .await?
                .check()?;

            Ok(())
        }

        async fn save_sync_error(
            &self,
            client_id: ClientId,
            error: &SyncError,
        ) -> Result<(), InfrastructureError> {
            self.0
                .query("UPDATE type::thing('client_sync', $client_id) SET error=$error")
                .bind(("client_id", client_id))
                .bind(("error", error))
                .await?
                .check()?;

            Ok(())
        }
    }
}
