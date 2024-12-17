pub use monee_types::backoffice::actors::*;
pub mod repository {
    use monee_core::ActorId;

    use crate::{
        prelude::AppError,
        shared::{domain::errors::UniqueSaveError, infrastructure::errors::InfrastructureError},
    };

    use super::{actor::Actor, actor_alias::ActorAlias};

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(&self, id: ActorId, actor: Actor) -> Result<(), AppError<UniqueSaveError>>;

        async fn alias_resolve(
            &self,
            name: &ActorAlias,
        ) -> Result<Option<ActorId>, InfrastructureError>;

        async fn get_all(&self) -> Result<Vec<(ActorId, Actor)>, InfrastructureError>;
    }
}