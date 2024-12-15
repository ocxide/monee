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
    use cream::event_bus::EventBusPort;
    use monee_types::apps::app_id::AppId;

    use crate::backoffice::events::domain::{
        apply_event::apply_event, repository::Repository as EventsRepository,
    };
    use crate::backoffice::snapshot::application::snapshot_io::SnapshotIO;
    use crate::host::sync::domain::client_synced::ClientSynced;
    use crate::host::sync::domain::sync_error::SyncError;
    use crate::host::sync::domain::{repository::Repository, sync_save::SyncSave};
    use crate::{iprelude::*, prelude::*};

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct DoSync {
        sync_repo: Box<dyn Repository>,
        snapshot_io: SnapshotIO,
        events_repo: Box<dyn EventsRepository>,
        event_bus: EventBusPort,
    }

    impl DoSync {
        pub async fn run(
            &self,
            client_id: AppId,
            sync: SyncSave,
        ) -> Result<(), AppError<SyncError>> {
            self.sync_repo.save_sync(client_id, &sync).await?;

            let mut snapshot = self.snapshot_io.read_last().await?;
            let events_apply_result = sync
                .events
                .iter()
                .map(|entry| &entry.event)
                .try_for_each(|event| apply_event(&mut snapshot, event));

            if let Err(e) = events_apply_result {
                let error = SyncError::Event(e);
                self.sync_repo.save_sync_error(client_id, &error).await?;
                return Err(AppError::App(error));
            }

            let save_result = self
                .sync_repo
                .save_changes(&sync.data)
                .await
                .catch_infra()?;

            if let Err(e) = save_result {
                let error = SyncError::Save(e);
                self.sync_repo.save_sync_error(client_id, &error).await?;
                return Err(AppError::App(error));
            }

            self.events_repo.save_many(sync.events).await?;

            self.event_bus.publish(ClientSynced(client_id));

            Ok(())
        }
    }
}

pub mod get_sync_report {
    use monee_types::host::sync::sync_report::SyncReport;

    use crate::iprelude::*;
    use crate::{backoffice::snapshot::application::snapshot_io::SnapshotIO, prelude::*};

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetSyncReport {
        snapshot_io: SnapshotIO,
        sync_repo: Box<dyn crate::host::sync::domain::repository::Repository>,
    }

    impl GetSyncReport {
        pub async fn run(&self) -> Result<SyncReport, InfrastructureError> {
            let snapshot = self.snapshot_io.read_last().await?;
            let data = self.sync_repo.get_context_data().await?;
            Ok(SyncReport { snapshot, data })
        }
    }
}

pub mod rewrite_system {
    use monee_types::host::sync::sync_report::SyncReport;
    use monee_types::shared::errors::UniqueSaveError;

    use crate::backoffice::snapshot::application::snapshot_io::SnapshotIO;
    use crate::{iprelude::*, prelude::*};

    use crate::host::sync::domain::repository::Repository;

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct RewriteSystem {
        repo: Box<dyn Repository>,
        snapshot_io: SnapshotIO,
    }

    impl RewriteSystem {
        pub async fn run(&self, data: SyncReport) -> Result<(), AppError<UniqueSaveError>> {
            self.repo.save_changes(&data.data).await?;
            self.snapshot_io.save(data.snapshot).await?;

            self.repo.truncate_events().await?;

            Ok(())
        }
    }
}
