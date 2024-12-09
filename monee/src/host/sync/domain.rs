pub mod sync_guide {
    use crate::shared::domain::date::Datetime;

    #[derive(serde::Serialize)]
    pub struct SyncGuide {
        pub last_event_date: Option<Datetime>,
    }
}

pub mod repository {
    use crate::prelude::InfrastructureError;

    use super::sync_guide::SyncGuide;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn get_sync_guide(&self) -> Result<SyncGuide, InfrastructureError>;
    }
}
