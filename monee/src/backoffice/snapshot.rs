pub mod domain {
    pub mod repository {
        use monee_core::Snapshot;

        use crate::shared::infrastructure::errors::InfrastructureError;

        #[async_trait::async_trait]
        pub trait SnapshotRepository: Send + Sync {
            async fn read(&self) -> Result<Snapshot, InfrastructureError>;
            async fn save(&self, snapshot: &Snapshot) -> Result<(), InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod on_wallet_created {
        use cream::{context::ContextProvide, events::Handler};

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::domain::context::AppContext,
        };

        use super::snapshot_read::SnapshotRead;

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct OnWalletCreated {
            snapshot_read: SnapshotRead,
            repository: Box<dyn SnapshotRepository>,
        }

        impl Handler for OnWalletCreated {
            type Event = crate::backoffice::wallets::domain::wallet_created::WalletCreated;

            async fn handle(&self, event: Self::Event) -> Result<(), cream::events::Error> {
                let mut snapshot = self.snapshot_read.read().await.expect("to read snapshot");

                // If snapshot already has this wallet, do nothing
                let result = snapshot.apply(monee_core::Operation::Wallet(
                    monee_core::WalletOperation::Create {
                        currency: event.currency_id,
                        wallet_id: event.id,
                    },
                ));

                if result.is_ok() {
                    self.repository
                        .save(&snapshot)
                        .await
                        .expect("to save snapshot");
                }

                Ok(())
            }
        }
    }

    pub mod snapshot_read {
        use cream::context::ContextProvide;

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct SnapshotRead {
            repository: Box<dyn SnapshotRepository>,
        }

        impl SnapshotRead {
            pub async fn read(&self) -> Result<monee_core::Snapshot, InfrastructureError> {
                // TODO: sync the snapshot with new events
                self.repository.read().await
            }
        }
    }
}

pub mod infrastructure {
    pub mod snapshot_repository {
        use cream::context::ContextProvide;

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{
                domain::context::DbContext,
                infrastructure::{database::Connection, errors::InfrastructureError},
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SnapshotSurrealRepository(Connection);

        #[async_trait::async_trait]
        impl SnapshotRepository for SnapshotSurrealRepository {
            async fn read(&self) -> Result<monee_core::Snapshot, InfrastructureError> {
                todo!()
            }

            async fn save(&self, _snapshot: &monee_core::Snapshot) -> Result<(), InfrastructureError> {
                todo!()
            }
        }
    }
}
