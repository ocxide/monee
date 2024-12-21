pub mod domain {
    pub mod repository {
        use monee_types::{
            host::sync::{sync_guide::SyncGuide, node_changes::EventEntry},
            nodes::sync::{changes_record::ChangesRecord, sync_context_data::Catalog},
            shared::errors::UniqueSaveError,
        };

        use crate::prelude::{AppError, InfrastructureError};

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn truncate_events(&self) -> Result<(), InfrastructureError>;
            async fn save_changes(
                &self,
                data: &Catalog,
            ) -> Result<(), AppError<UniqueSaveError>>;
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

pub mod infrastructure {
    pub mod repository {
        use crate::{
            host::sync::infrastructure::repository::save_changes,
            iprelude::*,
            nodes::sync::domain::repository::Repository,
            prelude::*,
            shared::{
                domain::context::DbContext,
                infrastructure::database::{Connection, Entity, EntityKey},
            },
        };
        use monee_core::CurrencyId;
        use monee_types::{
            backoffice::{
                actors::actor::Actor, currencies::currency::Currency, wallets::wallet::Wallet,
            },
            host::sync::node_changes::EventEntry,
            shared::errors::UniqueSaveError,
        };

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
                data: &monee_types::host::sync::catalog::Catalog,
            ) -> Result<(), AppError<UniqueSaveError>> {
                save_changes(&self.0, data).await
            }

            async fn get_catalog(
                &self,
                changes: &monee_types::nodes::sync::changes_record::ChangesRecord,
            ) -> Result<
                monee_types::host::sync::catalog::Catalog,
                InfrastructureError,
            > {
                let mut response = self
                    .0
                    .query("SELECT * FROM $currencies")
                    .bind((
                        "currencies",
                        changes
                            .currencies
                            .iter()
                            .copied()
                            .map(EntityKey)
                            .collect::<Vec<_>>(),
                    ))
                    .query("SELECT * FROM $actors")
                    .bind((
                        "actors",
                        changes
                            .actors
                            .iter()
                            .copied()
                            .map(EntityKey)
                            .collect::<Vec<_>>(),
                    ))
                    .query("SELECT * FROM $wallets")
                    .bind((
                        "wallets",
                        changes
                            .wallets
                            .iter()
                            .copied()
                            .map(EntityKey)
                            .collect::<Vec<_>>(),
                    ))
                    .await?;

                let currencies: Vec<Entity<CurrencyId, Currency>> = response.take(0)?;
                let actors: Vec<Entity<monee_core::ActorId, Actor>> = response.take(1)?;
                let wallets: Vec<Entity<monee_core::WalletId, Wallet>> = response.take(2)?;

                Ok(
                    monee_types::host::sync::catalog::Catalog {
                        currencies: currencies.into_iter().map(Into::into).collect(),
                        actors: actors.into_iter().map(Into::into).collect(),
                        wallets: wallets.into_iter().map(Into::into).collect(),
                        // TODO
                        items: vec![],
                    },
                )
            }

            async fn get_events(
                &self,
                guide: monee_types::host::sync::sync_guide::SyncGuide,
            ) -> Result<Vec<EventEntry>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT * FROM event WHERE date > $date")
                    .bind(("date", guide.last_event_date))
                    .await?;

                Ok(response.take(0)?)
            }
        }
    }
}

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
                self.repo.save_changes(&data.data).await?;
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
