pub mod sync_guide {
    use crate::shared::date::Datetime;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SyncGuide {
        pub last_event_date: Option<Datetime>,
    }
}

pub mod sync_save {
    use monee_core::EventId;

    use crate::{backoffice::events::event::Event, shared::date::Datetime};

    use super::sync_context_data::SyncContextData;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SyncSave {
        pub events: Vec<EventEntry>,
        #[serde(flatten)]
        pub data: SyncContextData,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct EventEntry {
        pub id: EventId,
        #[serde(flatten)]
        pub event: Event,
        pub created_at: Datetime,
    }
}

pub mod sync_context_data {
    use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};

    use crate::backoffice::{
        actors::actor::Actor, currencies::currency::Currency, item_tags::item_tag::ItemTag,
        wallets::wallet::Wallet,
    };

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct SyncContextData {
        pub actors: Vec<(ActorId, Actor)>,
        pub currencies: Vec<(CurrencyId, Currency)>,
        pub items: Vec<(ItemTagId, ItemTag)>,
        pub wallets: Vec<(WalletId, Wallet)>,
    }
}

pub mod sync_report {
    use monee_core::Snapshot;

    use super::sync_context_data::SyncContextData;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct SyncReport {
        pub snapshot: Snapshot,
        #[serde(flatten)]
        pub data: SyncContextData,
    }
}

pub mod sync_error {
    use crate::{
        backoffice::events::apply_event::Error as ApplyError, shared::errors::UniqueSaveError,
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

    use crate::apps::app_id::AppId;

    pub struct ClientSynced(pub AppId);
    impl DomainEvent for ClientSynced {
        fn name(&self) -> &'static str {
            "ClientSynced"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
