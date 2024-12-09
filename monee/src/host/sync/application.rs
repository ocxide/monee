pub mod get_sync_guide {
    use crate::host::sync::domain::{repository::Repository, sync_guide::SyncGuide};
    use crate::{iprelude::*, prelude::*};

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetSyncGuide(Box<dyn Repository>);

    impl GetSyncGuide {
        pub async fn run(&self) -> Result<SyncGuide, InfrastructureError> {
            self.0.get_sync_guide().await
        }
    }
}
