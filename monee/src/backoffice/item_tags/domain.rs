pub use monee_types::backoffice::item_tags::*;
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
            name: ItemName,
        ) -> Result<Option<ItemTagId>, InfrastructureError>;

        async fn get_all(&self) -> Result<Vec<ItemTagNode>, InfrastructureError>;
    }

    pub enum TagsRelation {
        Ancestor,
        NotRelated,
        TargetNotFound,
    }
}

