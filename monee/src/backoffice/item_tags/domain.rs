pub mod repository {
    use monee_core::ItemTagId;

    use crate::{
        prelude::AppError,
        shared::{domain::errors::UniqueSaveError, infrastructure::errors::InfrastructureError},
    };

    use super::{item_name::ItemName, item_tag::ItemTag, item_tag_node::ItemTagNode};

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(&self, id: ItemTagId, tag: ItemTag) -> Result<(), AppError<UniqueSaveError>>;

        async fn check_relation(
            &self,
            target_tag: ItemTagId,
            maybe_acestor: ItemTagId,
        ) -> Result<TagsRelation, InfrastructureError>;

        async fn link(
            &self,
            parent_id: ItemTagId,
            child_id: ItemTagId,
        ) -> Result<(), AppError<UniqueSaveError>>;

        async fn unlink(
            &self,
            parent_id: ItemTagId,
            child_id: ItemTagId,
        ) -> Result<(), InfrastructureError>;

        async fn name_resolve(
            &self,
            name: &ItemName,
        ) -> Result<Option<ItemTagId>, InfrastructureError>;

        async fn get_all(&self) -> Result<Vec<ItemTagNode>, InfrastructureError>;
    }

    pub enum TagsRelation {
        Ancestor,
        NotRelated,
        TargetNotFound,
    }
}

pub mod item_tag {
    use super::item_name::ItemName;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ItemTag {
        pub name: ItemName,
    }
}

pub mod item_name {
    use std::{fmt::Display, str::FromStr};

    use crate::shared::domain::alias::{from_str::Error, Alias};

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

    pub struct ItemTagNode {
        pub id: ItemTagId,
        pub tag: ItemTag,
        pub parents_name: Vec<ItemName>,
    }
}
