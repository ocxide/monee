pub use monee_types::backoffice::events::*;
pub mod repository {
    use crate::{
        host::sync::domain::sync_data::EventEntry,
        shared::infrastructure::errors::InfrastructureError,
    };

    use super::event::Event;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn add(&self, event: Event) -> Result<(), InfrastructureError>;
        async fn save_many(&self, events: Vec<EventEntry>) -> Result<(), InfrastructureError>;
    }
}