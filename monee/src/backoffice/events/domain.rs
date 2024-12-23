pub use monee_types::backoffice::events::*;
pub mod repository {
    use monee_core::EventId;

    use crate::{
        host::sync::domain::node_changes::EventEntry,
        shared::infrastructure::errors::InfrastructureError,
    };

    use super::event::Event;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn add(&self, id: EventId, event: Event) -> Result<(), InfrastructureError>;
        async fn save_many(&self, events: Vec<EventEntry>) -> Result<(), InfrastructureError>;
    }
}
