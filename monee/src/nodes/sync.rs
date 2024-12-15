pub mod domain {
    pub mod repository {
        use monee_types::{
            nodes::sync::sync_context_data::SyncContextData, shared::errors::UniqueSaveError,
        };

        use crate::prelude::{AppError, InfrastructureError};

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn truncate_events(&self) -> Result<(), InfrastructureError>;
            async fn save_changes(
                &self,
                data: &SyncContextData,
            ) -> Result<(), AppError<UniqueSaveError>>;
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use crate::{
            host::sync::infrastructure::repository::save_changes,
            iprelude::*,
            nodes::sync::domain::repository::Repository,
            prelude::*,
            shared::{domain::context::DbContext, infrastructure::database::Connection},
        };
        use monee_types::shared::errors::UniqueSaveError;

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn truncate_events(&self) -> Result<(), InfrastructureError> {
                self.0.query("DELETE event").await?.check()?;
                Ok(())
            }

            async fn save_changes(
                &self,
                data: &monee_types::host::sync::sync_context_data::SyncContextData,
            ) -> Result<(), AppError<UniqueSaveError>> {
                save_changes(&self.0, data).await
            }
        }
    }
}

pub mod application {
    pub mod rewrite_system {
        use monee_types::{
            host::sync::sync_report::SyncReport, shared::errors::UniqueSaveError,
        };

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
            pub async fn run(&self, data: SyncReport) -> Result<(), AppError<UniqueSaveError>> {
                self.repo.save_changes(&data.data).await?;
                self.snapshot_io.save(data.snapshot).await?;

                self.repo.truncate_events().await?;

                Ok(())
            }
        }
    }
}

