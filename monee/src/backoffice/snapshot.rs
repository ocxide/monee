pub mod domain {
    pub mod repository {
        use monee_core::Snapshot;

        use crate::shared::infrastructure::errors::InfrastructureError;

        #[async_trait::async_trait]
        pub trait SnapshotRepository: Send + Sync {
            async fn read_last(&self) -> Result<Option<Snapshot>, InfrastructureError>;
            async fn save(&self, snapshot: Snapshot) -> Result<(), InfrastructureError>;
            async fn delete_all(&self) -> Result<(), InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod on_wallet_created {
        use cream::{context::ContextProvide, events::Handler};

        use crate::shared::domain::context::AppContext;

        use super::snapshot_io::SnapshotIO;

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct OnWalletCreated {
            snapshot_io: SnapshotIO,
        }

        impl Handler for OnWalletCreated {
            type Event = crate::backoffice::wallets::domain::wallet_created::WalletCreated;

            async fn handle(&self, event: Self::Event) -> Result<(), cream::events::Error> {
                let mut snapshot = self
                    .snapshot_io
                    .read_last()
                    .await
                    .expect("to read snapshot");

                // If snapshot already has this wallet, do nothing
                let result = snapshot.apply(monee_core::Operation::Wallet(
                    monee_core::WalletOperation::Create {
                        currency_id: event.currency_id,
                        wallet_id: event.id,
                    },
                ));

                if result.is_ok() {
                    self.snapshot_io
                        .save(snapshot)
                        .await
                        .expect("to save snapshot");
                }

                Ok(())
            }
        }
    }

    pub mod snapshot_io {
        use cream::context::ContextProvide;

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct SnapshotIO {
            repository: Box<dyn SnapshotRepository>,
        }

        impl SnapshotIO {
            pub async fn read_last(&self) -> Result<monee_core::Snapshot, InfrastructureError> {
                self.repository
                    .read_last()
                    .await
                    .map(|snapshot| snapshot.unwrap_or_default())
            }

            pub async fn save(
                &self,
                snapshot: monee_core::Snapshot,
            ) -> Result<(), InfrastructureError> {
                self.repository.delete_all().await?;
                self.repository.save(snapshot).await?;

                Ok(())
            }
        }
    }
}

pub mod infrastructure {
    pub mod snapshot_repository {
        use cream::context::ContextProvide;
        use monee_core::{DebtId, MoneyMap, Snapshot, WalletId};

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{
                domain::context::DbContext,
                infrastructure::{
                    database::{Connection, Entity},
                    errors::InfrastructureError,
                },
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SnapshotSurrealRepository(Connection);

        #[derive(serde::Serialize, serde::Deserialize)]
        struct SurrealSnapshot {
            wallets: Vec<Entity<WalletId, monee_core::Wallet>>,
            debts: Vec<Entity<DebtId, monee_core::Debt>>,
            loans: Vec<Entity<DebtId, monee_core::Debt>>,
        }

        impl From<Snapshot> for SurrealSnapshot {
            fn from(snapshot: Snapshot) -> Self {
                Self {
                    wallets: snapshot.wallets.into_iter().map(Entity::from).collect(),
                    debts: snapshot.debts.into_iter().map(Entity::from).collect(),
                    loans: snapshot.loans.into_iter().map(Entity::from).collect(),
                }
            }
        }

        impl From<SurrealSnapshot> for Snapshot {
            fn from(snapshot: SurrealSnapshot) -> Self {
                Self {
                    // Build from surraldb, its supossed to be valid data
                    wallets: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.wallets.into_iter().map(Entity::into_inner),
                        )
                    },
                    // Build from surraldb, its supossed to be valid data
                    debts: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.debts.into_iter().map(Entity::into_inner),
                        )
                    },
                    // Build from surraldb, its supossed to be valid data
                    loans: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.loans.into_iter().map(Entity::into_inner),
                        )
                    },
                }
            }
        }

        #[async_trait::async_trait]
        impl SnapshotRepository for SnapshotSurrealRepository {
            async fn read_last(&self) -> Result<Option<Snapshot>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT * FROM ONLY snapshot ORDER BY created_at DESC LIMIT 1")
                    .await?
                    .check()?;

                let snapshot: Option<SurrealSnapshot> = response.take(0)?;
                Ok(snapshot.map(From::from))
            }

            async fn save(
                &self,
                snapshot: monee_core::Snapshot,
            ) -> Result<(), InfrastructureError> {
                self.0
                    .query("CREATE snapshot CONTENT $snapshot")
                    .bind(("snapshot", SurrealSnapshot::from(snapshot)))
                    .await?
                    .check()?;

                Ok(())
            }

            async fn delete_all(&self) -> Result<(), InfrastructureError> {
                self.0.query("DELETE FROM snapshot").await?.check()?;
                Ok(())
            }
        }
    }
}
