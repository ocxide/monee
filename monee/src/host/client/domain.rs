pub use monee_types::host::client::*;
pub mod repository {
    use crate::prelude::InfrastructureError;

    use super::{client::Client, client_id::ClientId};

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(&self, id: ClientId, client: Client) -> Result<(), InfrastructureError>;

        async fn exists(&self, id: ClientId) -> Result<bool, InfrastructureError>;
    }
}