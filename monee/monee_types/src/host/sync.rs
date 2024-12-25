pub mod sync_guide {
    use crate::shared::date::Datetime;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SyncGuide {
        pub last_event_date: Option<Datetime>,
    }
}

pub mod node_changes {
    use monee_core::EventId;

    use crate::{backoffice::events::event::Event, shared::date::Datetime};

    use super::catalog::Catalog;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct NodeChanges {
        pub events: Vec<EventEntry>,
        #[serde(flatten)]
        pub data: Catalog,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct EventEntry {
        pub id: EventId,
        #[serde(flatten)]
        pub event: Event,
        pub created_at: Datetime,
    }
}

pub mod catalog {
    use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};

    use crate::backoffice::{
        actors::actor::Actor, currencies::currency::Currency, item_tags::item_tag::ItemTag,
        wallets::wallet::Wallet,
    };

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct Catalog {
        pub actors: Vec<(ActorId, Actor)>,
        pub currencies: Vec<(CurrencyId, Currency)>,
        pub items: Vec<(ItemTagId, ItemTag)>,
        pub wallets: Vec<(WalletId, Wallet)>,
    }
}

pub mod host_state {
    use monee_core::Snapshot;

    use super::catalog::Catalog;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct HostState {
        pub snapshot: Snapshot,
        #[serde(flatten)]
        pub data: Catalog,
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

pub mod node_synced {
    use cream_events_core::DomainEvent;

    use crate::apps::app_id::AppId;

    pub struct NodeSynced(pub AppId);
    impl DomainEvent for NodeSynced {
        fn name(&self) -> &'static str {
            "NodeSynced"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
