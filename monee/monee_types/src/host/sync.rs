pub mod sync_guide {
    use crate::shared::date::Datetime;

    #[derive(serde::Serialize)]
    pub struct SyncGuide {
        pub last_event_date: Option<Datetime>,
    }
}

pub mod sync_data {
    use monee_core::{ActorId, CurrencyId, EventId, ItemTagId, Wallet, WalletId};

    use crate::{
        backoffice::{
            actors::actor::Actor, currencies::currency::Currency,
            events::event::Event, item_tags::item_tag::ItemTag,
        },
        shared::date::Datetime,
    };

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SyncData {
        pub events: Vec<EventEntry>,
        pub actors: Vec<Entry<ActorId, Actor>>,
        pub currencies: Vec<Entry<CurrencyId, Currency>>,
        pub items: Vec<Entry<ItemTagId, ItemTag>>,
        pub wallets: Vec<Entry<WalletId, Wallet>>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct EventEntry {
        pub id: EventId,
        #[serde(flatten)]
        pub event: Event,
        pub created_at: Datetime,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Entry<K, T> {
        pub id: K,
        #[serde(flatten)]
        pub data: T,
    }
}

pub mod sync_error {
    use crate::{
        backoffice::events::apply_event::Error as ApplyError,
        shared::errors::UniqueSaveError,
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
    use cream_events_core::DomainEvent;

    use crate::host::client::client_id::ClientId;

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


