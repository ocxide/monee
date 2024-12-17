pub use monee_types::host::sync::*;
pub mod repository {

    use monee_types::apps::app_id::AppId;

    use crate::{
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
            app_id: AppId,
            sync: &SyncSave,
        ) -> Result<(), InfrastructureError>;

        async fn save_sync_error(
            &self,
            app_id: AppId,
            error: &SyncError,
        ) -> Result<(), InfrastructureError>;

        async fn save_changes(
            &self,
            data: &SyncContextData,
        ) -> Result<(), AppError<UniqueSaveError>>;

        async fn get_context_data(&self) -> Result<SyncContextData, InfrastructureError>;
    }
}
