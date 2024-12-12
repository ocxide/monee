pub use monee_types::host::sync::*;
pub mod repository {
    use monee_core::{ActorId, CurrencyId, ItemTagId, Wallet, WalletId};

    use crate::{
        backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
            item_tags::domain::item_tag::ItemTag,
        },
        host::client::domain::client_id::ClientId,
        prelude::{AppError, InfrastructureError},
        shared::domain::errors::UniqueSaveError,
    };

    use super::{
        sync_data::{Entry, SyncData},
        sync_error::SyncError,
        sync_guide::SyncGuide,
    };

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn get_sync_guide(&self) -> Result<SyncGuide, InfrastructureError>;

        async fn save_sync(
            &self,
            client_id: ClientId,
            sync: &SyncData,
        ) -> Result<(), InfrastructureError>;

        async fn save_sync_error(
            &self,
            client_id: ClientId,
            error: &SyncError,
        ) -> Result<(), InfrastructureError>;

        async fn save_changes(
            &self,
            currencies: &[Entry<CurrencyId, Currency>],
            items: &[Entry<ItemTagId, ItemTag>],
            actors: &[Entry<ActorId, Actor>],
            wallets: &[Entry<WalletId, Wallet>],
        ) -> Result<(), AppError<UniqueSaveError>>;
    }
}