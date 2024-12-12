pub use monee_types::reports::events::*;
pub mod repository {
    use crate::shared::infrastructure::errors::InfrastructureError;

    use super::event::Event;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn get_all(&self) -> Result<Vec<Event>, InfrastructureError>;
    }
}