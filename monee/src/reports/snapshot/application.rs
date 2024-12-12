pub mod snapshot_report {
    use cream::context::FromContext;

    use crate::{
        reports::snapshot::domain::{repository::Repository, snapshot::Snapshot},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct SnapshotReport {
        repository: Box<dyn Repository>,
    }

    impl SnapshotReport {
        pub async fn run(&self) -> Result<Snapshot, InfrastructureError> {
            // TODO: ensure the snapshot is up-to-date
            self.repository.read().await
        }
    }
}

