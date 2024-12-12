pub use monee_types::reports::snapshot::*;
pub mod repository {
    use crate::shared::infrastructure::errors::InfrastructureError;

    use super::snapshot::Snapshot;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn read(&self) -> Result<Snapshot, InfrastructureError>;
    }
}