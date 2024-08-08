pub mod actors;
pub mod currencies;
pub mod events;
pub mod item_tags;
pub mod wallets;
pub mod shared {
    pub mod domain {
        pub(crate) mod snapshot_repository {
            use cream::context::FromContext;
            use monee_core::Snapshot;

            use crate::shared::{
                domain::context::AppContext, infrastructure::errors::UnspecifiedError,
            };

            impl<C: AppContext> FromContext<C> for Box<dyn SnapshotRepository> {
                fn from_context(ctx: &C) -> Self {
                    ctx.backoffice_shared_snapshot_repository()
                }
            }

            #[async_trait::async_trait]
            pub trait SnapshotRepository {
                async fn read(&self) -> Result<Snapshot, UnspecifiedError>;
                async fn save(&self, snapshot: &Snapshot) -> Result<(), UnspecifiedError>;
            }
        }
    }

    pub mod infrastructure {
        pub(crate) mod snapshot_repository {
            use crate::{
                backoffice::shared::domain::snapshot_repository::SnapshotRepository,
                shared::infrastructure::{database::Connection, errors::UnspecifiedError},
            };

            pub struct SnapshotSurrealRepository(Connection);

            impl SnapshotSurrealRepository {
                pub(crate) fn new(connection: Connection) -> Self {
                    Self(connection)
                }
            }

            #[async_trait::async_trait]
            impl SnapshotRepository for SnapshotSurrealRepository {
                async fn read(&self) -> Result<monee_core::Snapshot, UnspecifiedError> {
                    todo!()
                }

                async fn save(
                    &self,
                    _snapshot: &monee_core::Snapshot,
                ) -> Result<(), UnspecifiedError> {
                    todo!()
                }
            }
        }
    }
}
