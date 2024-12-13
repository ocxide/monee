pub use monee_types::host::sync::*;
pub mod repository {

    use crate::{
        host::client::domain::client_id::ClientId,
        prelude::{AppError, InfrastructureError},
        shared::domain::errors::UniqueSaveError,
    };

    use super::{
        sync_context_data::SyncContextData, sync_error::SyncError, sync_guide::SyncGuide,
        sync_save::SyncSave,
    };

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn get_sync_guide(&self) -> Result<SyncGuide, InfrastructureError>;

        async fn save_sync(
            &self,
            client_id: ClientId,
            sync: &SyncSave,
        ) -> Result<(), InfrastructureError>;

        async fn save_sync_error(
            &self,
            client_id: ClientId,
            error: &SyncError,
        ) -> Result<(), InfrastructureError>;

        async fn save_changes(
            &self,
            data: &SyncContextData,
        ) -> Result<(), AppError<UniqueSaveError>>;

        async fn get_context_data(&self) -> Result<SyncContextData, InfrastructureError>;
    }
}
