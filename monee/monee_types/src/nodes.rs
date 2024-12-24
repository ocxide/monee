pub mod host {
    pub mod host_dir {
        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
        pub struct HostDir(String);

        impl std::fmt::Display for HostDir {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl AsRef<str> for HostDir {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for HostDir {
            fn from(value: String) -> Self {
                HostDir(value)
            }
        }
    }

    pub mod host_binding {
        use crate::apps::app_id::AppId;

        use super::host_dir::HostDir;

        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
        pub struct HostBinding {
            pub dir: HostDir,
            pub node_app_id: AppId,
        }
    }
}

pub mod sync {
    pub mod sync_context_data {
        pub use crate::host::sync::catalog::Catalog;
    }

    pub mod sync_report {
        pub use crate::host::sync::host_state::HostState;
    }

    pub mod changes_record {
        use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};

        #[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
        pub struct ChangesRecord {
            pub currencies: Vec<CurrencyId>,
            pub actors: Vec<ActorId>,
            pub wallets: Vec<WalletId>,
            pub items: Vec<ItemTagId>,
        }
    }

    pub mod sync_save {
        pub use crate::host::sync::node_changes::NodeChanges;
    }
}
