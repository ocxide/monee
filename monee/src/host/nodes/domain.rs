pub use monee_types::apps::*;
pub mod repository {
    use monee_types::apps::{app_id::AppId, app_manifest::AppManifest};

    use crate::prelude::InfrastructureError;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(&self, id: AppId, app: AppManifest) -> Result<(), InfrastructureError>;

        async fn exists(&self, id: AppId) -> Result<bool, InfrastructureError>;
    }
}

