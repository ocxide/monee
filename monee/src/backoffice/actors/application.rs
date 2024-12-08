pub mod create_one {
    use cream::context::FromContext;
    use monee_core::ActorId;

    use crate::{
        backoffice::actors::domain::{actor::Actor, repository::Repository},
        prelude::AppError,
        shared::domain::{context::AppContext, errors::UniqueSaveError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct CreateOne {
        repository: Box<dyn Repository>,
    }

    impl CreateOne {
        pub async fn run(&self, actor: Actor) -> Result<(), AppError<UniqueSaveError>> {
            self.repository.save(ActorId::new(), actor).await
        }
    }
}

pub mod alias_resolve {
    use cream::context::FromContext;
    use monee_core::ActorId;

    use crate::{
        backoffice::actors::domain::{actor_alias::ActorAlias, repository::Repository},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct AliasResolve {
        repository: Box<dyn Repository>,
    }

    impl AliasResolve {
        pub async fn run(&self, name: &ActorAlias) -> Result<Option<ActorId>, InfrastructureError> {
            self.repository.alias_resolve(name).await
        }
    }
}

pub mod get_all {
    use cream::context::FromContext;
    use monee_core::ActorId;

    use crate::{
        backoffice::actors::domain::{actor::Actor, repository::Repository},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetAll {
        repository: Box<dyn Repository>,
    }

    impl GetAll {
        pub async fn run(&self) -> Result<Vec<(ActorId, Actor)>, InfrastructureError> {
            self.repository.get_all().await
        }
    }
}