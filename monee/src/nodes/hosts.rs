pub mod domain {
    pub use monee_types::nodes::*;

    pub mod repository {
        use monee_types::nodes::host::host_binding::HostBinding;

        use crate::prelude::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn save_host_binding(
                &self,
                host_binding: &HostBinding,
            ) -> Result<(), InfrastructureError>;
            async fn get_host_binding(&self) -> Result<Option<HostBinding>, InfrastructureError>;
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use super::super::domain::repository::Repository;
        use crate::shared::{domain::context::DbContext, infrastructure::database::Connection};
        use crate::{iprelude::*, prelude::*};
        use monee_types::nodes::host::host_binding::HostBinding;

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save_host_binding(
                &self,
                host_binding: &HostBinding,
            ) -> Result<(), InfrastructureError> {
                self.0
                    .query("UPDATE host_binding:me SET dir = $dir, node_app_id = $node_app_id")
                    .bind(host_binding)
                    .await?;

                Ok(())
            }

            async fn get_host_binding(&self) -> Result<Option<HostBinding>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT dir, node_app_id FROM host_binding LIMIT 1")
                    .await?;

                let host_dir: Option<HostBinding> = response.take(0)?;
                Ok(host_dir)
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[cfg(feature = "db_test")]
            #[tokio::test]
            async fn it_saves() {
                use monee_types::{apps::app_id::AppId, nodes::host::host_binding::HostBinding};
                use cream::context::Context;

                let con = crate::shared::infrastructure::database::connect()
                    .await
                    .unwrap();
                let ctx= DbContext::new(con);

                let app_id = AppId::default();
                let og_host_binding = HostBinding {
                    dir: "/tmp/monee".to_owned().into(), 
                    node_app_id: app_id
                };

                let repo: SurrealRepository = ctx.provide();
                repo.save_host_binding(&og_host_binding).await.unwrap();

                let host_binding = repo.get_host_binding().await.unwrap();
                assert_eq!(host_binding, Some(og_host_binding));
            }
        }
    }
}

pub mod application {
    pub mod save_host_dir {
        use monee_types::nodes::host::host_binding::HostBinding;

        use super::super::domain::repository::Repository;
        use crate::{iprelude::*, prelude::*};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct SetHostBinding {
            repository: Box<dyn Repository>,
        }

        impl SetHostBinding {
            pub async fn run(&self, host_binding: &HostBinding) -> Result<(), InfrastructureError> {
                self.repository.save_host_binding(host_binding).await
            }
        }
    }

    pub mod get_host_info {
        use monee_types::nodes::host::host_binding::HostBinding;

        use super::super::domain::repository::Repository;
        use crate::{iprelude::*, prelude::*};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct GetHostBinding {
            repository: Box<dyn Repository>,
        }

        impl GetHostBinding {
            pub async fn run(&self) -> Result<Option<HostBinding>, InfrastructureError> {
                self.repository.get_host_binding().await
            }
        }
    }
}
