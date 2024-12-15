pub mod domain {
    pub use monee_types::nodes::*;

    pub mod repository {
        use monee_types::nodes::host_dir::HostDir;

        use crate::prelude::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn save_host_dir(&self, host_dir: HostDir) -> Result<(), InfrastructureError>;
            async fn get_host_dir(&self) -> Result<Option<HostDir>, InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod save_host_dir {
        use monee_types::nodes::host_dir::HostDir;

        use crate::iprelude::*;
        use crate::nodes::domain::repository::Repository;
        use crate::prelude::InfrastructureError;

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct SaveHostDir {
            repository: Box<dyn Repository>,
        }

        impl SaveHostDir {
            pub async fn run(&self, host_dir: HostDir) -> Result<(), InfrastructureError> {
                self.repository.save_host_dir(host_dir).await
            }
        }
    }

    pub mod get_host_dir {
        use monee_types::nodes::host_dir::HostDir;

        use crate::iprelude::*;
        use crate::nodes::domain::repository::Repository;
        use crate::prelude::InfrastructureError;

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct GetHostDir {
            repository: Box<dyn Repository>,
        }

        impl GetHostDir {
            pub async fn run(&self) -> Result<Option<HostDir>, InfrastructureError> {
                self.repository.get_host_dir().await
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use crate::iprelude::*;
        use crate::nodes::domain::repository::Repository;
        use crate::prelude::*;
        use crate::shared::domain::context::DbContext;
        use crate::shared::infrastructure::database::Connection;
        use monee_types::nodes::host_dir::HostDir;

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save_host_dir(&self, host_dir: HostDir) -> Result<(), InfrastructureError> {
                self.0
                    .query("UPDATE self_app SET host_dir = $host_dir")
                    .bind(("host_dir", host_dir))
                    .await
                    .catch_infra()
            }

            async fn get_host_dir(&self) -> Result<Option<HostDir>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT host_dir FROM self_app LIMIT 1")
                    .await
                    .catch_infra()?;

                let host_dir: Option<HostDir> = response.take((0, "host_dir"))?;
                Ok(host_dir)
            }
        }
    }
}
