pub mod application {
    pub mod get_self {
        use monee_types::apps::app_id::AppId;

        use crate::{iprelude::*, prelude::*, self_config::domain::repository::Repository};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct GetSelf {
            repository: Box<dyn Repository>,
        }

        impl GetSelf {
            pub async fn run(&self) -> Result<Option<AppId>, InfrastructureError> {
                self.repository.get_self().await
            }
        }
    }

    pub mod save_self {
        use monee_types::apps::app_id::AppId;

        use crate::iprelude::*;
        use crate::prelude::*;
        use crate::self_config::domain::repository::Repository;

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct SaveSelf {
            repository: Box<dyn Repository>,
        }

        impl SaveSelf {
            pub async fn run(&self, id: AppId) -> Result<(), InfrastructureError> {
                self.repository.save_self(id).await
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_types::apps::app_id::AppId;

        use crate::prelude::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn get_self(&self) -> Result<Option<AppId>, InfrastructureError>;
            async fn save_self(&self, id: AppId) -> Result<(), InfrastructureError>;
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::FromContext;
        use monee_types::apps::app_id::AppId;

        use crate::{
            iprelude::CatchInfra,
            prelude::InfrastructureError,
            self_config::domain::repository::Repository,
            shared::{
                domain::context::DbContext,
                infrastructure::database::{Connection, Entity},
            },
        };

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn get_self(&self) -> Result<Option<AppId>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT id FROM self_app LIMIT 1")
                    .await
                    .catch_infra()?;

                let id: Option<Entity<AppId, ()>> = response.take(0)?;
                Ok(id.map(Entity::into_key))
            }

            async fn save_self(&self, id: AppId) -> Result<(), InfrastructureError> {
                self.0
                    .query("CREATE self_app SET id = $id")
                    .bind(("id", id))
                    .await
                    .catch_infra()?;

                Ok(())
            }
        }
    }
}
