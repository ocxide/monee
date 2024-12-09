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

pub mod do_sync {
    use crate::backoffice::events::domain::{
        apply_event::apply_event, repository::Repository as EventsRepository,
    };
    use crate::backoffice::snapshot::application::snapshot_io::SnapshotIO;
    use crate::host::client::domain::client_id::ClientId;
    use crate::host::sync::domain::sync_error::SyncError;
    use crate::host::sync::domain::{repository::Repository, sync_data::SyncData};
    use crate::{iprelude::*, prelude::*};

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct DoSync {
        sync_repo: Box<dyn Repository>,
        snapshot_io: SnapshotIO,
        events_repo: Box<dyn EventsRepository>,
    }

    impl DoSync {
        pub async fn run(
            &self,
            client_id: ClientId,
            sync: SyncData,
        ) -> Result<(), AppError<SyncError>> {
            self.sync_repo.save_sync(client_id, &sync).await?;

            let mut snapshot = self.snapshot_io.read_last().await?;
            let events_apply_result = sync
                .events
                .iter()
                .map(|(_, entry)| &entry.event)
                .try_for_each(|event| apply_event(&mut snapshot, event));

            if let Err(e) = events_apply_result {
                let error = SyncError::Event(e);
                self.sync_repo.save_sync_error(client_id, &error).await?;
                return Err(AppError::App(error));
            }

            Ok(())
        }
    }
}
