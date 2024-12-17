pub mod create_one {
    use cream::context::FromContext;
    use monee_core::ItemTagId;

    use crate::{
        backoffice::item_tags::domain::{item_tag::ItemTag, repository::Repository},
        prelude::AppError,
        shared::domain::{context::AppContext, errors::UniqueSaveError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct CreateOne {
        repository: Box<dyn Repository>,
    }

    impl CreateOne {
        pub async fn run(&self, tag: ItemTag) -> Result<(), AppError<UniqueSaveError>> {
            let id = ItemTagId::new();
            self.repository.save(id, tag).await
        }
    }
}

pub mod link_one {
    use cream::context::FromContext;
    use monee_core::ItemTagId;

    use crate::{
        backoffice::item_tags::domain::repository::{Repository, TagsRelation},
        prelude::AppError,
        shared::{
            domain::{context::AppContext, errors::UniqueSaveError},
            infrastructure::errors::InfrastructureError,
        },
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct LinkOne {
        repository: Box<dyn Repository>,
    }

    impl LinkOne {
        pub async fn run(
            &self,
            parent_id: ItemTagId,
            child_id: ItemTagId,
        ) -> Result<Status, InfrastructureError> {
            if parent_id == child_id {
                return Ok(Status::CyclicRelation);
            }

            // Check if child_id is already parent of parent_id
            match self.repository.check_relation(parent_id, child_id).await? {
                TagsRelation::TargetNotFound => return Ok(Status::NotFound(parent_id)),
                TagsRelation::Ancestor => return Ok(Status::CyclicRelation),
                TagsRelation::NotRelated => {}
            };

            match self.repository.link(parent_id, child_id).await {
                Ok(_) => Ok(Status::Linked),
                Err(AppError::Infrastructure(e)) => Err(e),
                Err(AppError::App(UniqueSaveError::AlreadyExists)) => Ok(Status::AlreadyContains),
            }
        }
    }

    pub enum Status {
        Linked,
        AlreadyContains,
        CyclicRelation,
        NotFound(monee_core::ItemTagId),
    }
}

pub mod unlink {
    use cream::context::FromContext;
    use monee_core::ItemTagId;

    use crate::{
        backoffice::item_tags::domain::repository::Repository,
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct Unlink {
        repository: Box<dyn Repository>,
    }

    impl Unlink {
        pub async fn run(
            &self,
            parent_id: ItemTagId,
            child_id: ItemTagId,
        ) -> Result<(), InfrastructureError> {
            self.repository.unlink(parent_id, child_id).await
        }
    }
}

pub mod name_resolve {
    use cream::context::FromContext;
    use monee_core::ItemTagId;

    use crate::{
        backoffice::item_tags::domain::{item_name::ItemName, repository::Repository},
        prelude::{AppContext, InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct NameResolve {
        repository: Box<dyn Repository>,
    }

    impl NameResolve {
        pub async fn run(&self, name: &ItemName) -> Result<Option<ItemTagId>, InfrastructureError> {
            self.repository.name_resolve(name).await
        }
    }
}

pub mod get_all {
    use cream::context::FromContext;

    use crate::{
        backoffice::item_tags::domain::{item_tag_node::ItemTagNode, repository::Repository},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetAll {
        repository: Box<dyn Repository>,
    }

    impl GetAll {
        pub async fn run(&self) -> Result<Vec<ItemTagNode>, InfrastructureError> {
            self.repository.get_all().await
        }
    }
}
