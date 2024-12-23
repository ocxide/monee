pub mod item_tag {
    use super::item_name::ItemName;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ItemTag {
        pub name: ItemName,
    }
}

pub mod item_name {
    use std::{fmt::Display, str::FromStr};

    use crate::shared::alias::{from_str::Error, Alias};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ItemName(Alias);

    impl Display for ItemName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl FromStr for ItemName {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(Alias::from_str(s)?))
        }
    }
}

pub mod item_tag_node {
    use super::{item_name::ItemName, item_tag::ItemTag};
    use monee_core::ItemTagId;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ItemTagNode {
        pub id: ItemTagId,
        pub tag: ItemTag,
        pub parents_name: Vec<ItemName>,
    }
}

pub mod item_tag_created {
    use cream_events_core::DomainEvent;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct ItemTagCreated {
        pub id: monee_core::ItemTagId,
    }

    impl DomainEvent for ItemTagCreated {
        fn name(&self) -> &'static str {
            "backoffice.item_tags.created"
        }

        fn version(&self) -> &'static str {
            "1.0.0"
        }
    }
}
