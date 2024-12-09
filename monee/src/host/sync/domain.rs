pub mod sync_guide {
    use crate::shared::domain::date::Datetime;

    #[derive(serde::Serialize)]
    pub struct SyncGuide {
        pub last_event_date: Option<Datetime>,
    }
}

pub mod sync_data {
    use monee_core::{ActorId, CurrencyId, EventId, ItemTagId, Wallet, WalletId};

    use crate::{
        backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
            events::domain::event::Event, item_tags::domain::item_tag::ItemTag,
        },
        shared::domain::date::Datetime,
    };

    #[derive(serde::Serialize)]
    pub struct SyncData {
        pub events: Vec<(EventId, EventEntry)>,
        pub actors: Vec<(ActorId, Actor)>,
        pub currencies: Vec<(CurrencyId, Currency)>,
        pub items: Vec<(ItemTagId, ItemTag)>,
        pub wallets: Vec<(WalletId, Wallet)>,
    }

    #[derive(serde::Serialize)]
    pub struct EventEntry {
        pub event: Event,
        pub created_at: Datetime,
    }
}

pub mod sync_error {
    use crate::{
        backoffice::events::domain::apply_event::Error as ApplyError,
        shared::domain::errors::UniqueSaveError,
    };

    #[derive(serde::Serialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum SyncError {
        Event(ApplyError),
        Save(UniqueSaveError),
    }

    impl From<UniqueSaveError> for SyncError {
        fn from(error: UniqueSaveError) -> Self {
            SyncError::Save(error)
        }
    }
}

pub mod client_synced {
    use cream::events::DomainEvent;

    use crate::host::client::domain::client_id::ClientId;

    pub struct ClientSynced(pub ClientId);
    impl DomainEvent for ClientSynced {
        fn name(&self) -> &'static str {
            "ClientSynced"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}

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

    use super::{sync_data::SyncData, sync_error::SyncError, sync_guide::SyncGuide};

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
            currencies: &[(CurrencyId, Currency)],
            items: &[(ItemTagId, ItemTag)],
            actors: &[(ActorId, Actor)],
            wallets: &[(WalletId, Wallet)],
        ) -> Result<(), AppError<UniqueSaveError>>;
    }
}
