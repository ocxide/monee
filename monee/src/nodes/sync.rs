pub mod domain {
    pub mod repository {
        use monee_types::{
            host::sync::{node_changes::EventEntry, sync_guide::SyncGuide},
            nodes::sync::{changes_record::ChangesRecord, sync_context_data::Catalog},
            shared::errors::UniqueSaveError,
        };

        use crate::prelude::{AppError, InfrastructureError};

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn truncate_events(&self) -> Result<(), InfrastructureError>;
            async fn save_catalog(&self, data: &Catalog) -> Result<(), AppError<UniqueSaveError>>;
            async fn get_catalog(
                &self,
                changes: &ChangesRecord,
            ) -> Result<Catalog, InfrastructureError>;
            async fn get_events(
                &self,
                guide: SyncGuide,
            ) -> Result<Vec<EventEntry>, InfrastructureError>;
        }
    }
}

pub mod infrastructure;

pub mod application {
    pub mod rewrite_system {
        use monee_types::{nodes::sync::sync_report::HostState, shared::errors::UniqueSaveError};

        use crate::backoffice::snapshot::application::snapshot_io::SnapshotIO;

        use super::super::domain::repository::Repository;
        use crate::{iprelude::*, prelude::*};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct RewriteSystem {
            repo: Box<dyn Repository>,
            snapshot_io: SnapshotIO,
        }

        impl RewriteSystem {
            pub async fn run(&self, data: HostState) -> Result<(), AppError<UniqueSaveError>> {
                self.repo.save_catalog(&data.data).await?;
                self.snapshot_io.save(data.snapshot).await?;

                self.repo.truncate_events().await?;

                Ok(())
            }
        }
    }

    pub mod get_node_changes {
        use monee_types::{
            host::sync::sync_guide::SyncGuide,
            nodes::sync::{changes_record::ChangesRecord, sync_save::NodeChanges},
        };

        use super::super::domain::repository::Repository;
        use crate::{iprelude::*, prelude::*};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct GetNodeChanges {
            repo: Box<dyn Repository>,
        }

        impl GetNodeChanges {
            pub async fn run(
                &self,
                guide: SyncGuide,
                changes: &ChangesRecord,
            ) -> Result<NodeChanges, InfrastructureError> {
                let events = self.repo.get_events(guide).await?;
                let data = self.repo.get_catalog(changes).await?;
                Ok(NodeChanges { events, data })
            }
        }
    }
}
