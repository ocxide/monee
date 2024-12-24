pub mod application {
    pub mod load {
        use cream::context::FromContext;
        use monee_types::nodes::sync::changes_record::ChangesRecord;

        use crate::{nodes::changes::domain::repository::Repository, prelude::AppContext};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct Load {
            repo: Box<dyn Repository>,
        }

        impl Load {
            pub async fn run(
                &self,
            ) -> Result<ChangesRecord, crate::prelude::InfrastructureError> {
                self.repo
                    .load()
                    .await
                    .map(|changes| changes.unwrap_or_default())
            }
        }
    }

    pub mod save {
        use cream::context::FromContext;
        use monee_types::nodes::sync::changes_record::ChangesRecord;

        use crate::{nodes::changes::domain::repository::Repository, prelude::AppContext};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct Save {
            repo: Box<dyn Repository>,
        }

        impl Save {
            pub async fn run(
                &self,
                changes: &ChangesRecord,
            ) -> Result<(), crate::prelude::InfrastructureError> {
                self.repo.save(changes).await
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_types::nodes::sync::changes_record::ChangesRecord;

        use crate::prelude::InfrastructureError;

        #[async_trait::async_trait]
        pub trait Repository: Send + Sync + 'static {
            async fn save(&self, changes: &ChangesRecord) -> Result<(), InfrastructureError>;
            async fn load(&self) -> Result<Option<ChangesRecord>, InfrastructureError>;
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::FromContext;
        use monee_types::nodes::sync::changes_record::ChangesRecord;

        use crate::{
            nodes::changes::domain::repository::Repository,
            prelude::*,
            shared::{domain::context::DbContext, infrastructure::database::Connection},
        };

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, changes: &ChangesRecord) -> Result<(), InfrastructureError> {
                self.0
                    .query("UPDATE changes_record:one CONTENT $data")
                    .bind(("data", changes))
                    .await?;

                Ok(())
            }

            async fn load(&self) -> Result<Option<ChangesRecord>, InfrastructureError> {
                let mut response = self.0.query("SELECT * FROM changes_record").await?;

                let changes: Option<ChangesRecord> = response.take(0)?;
                Ok(changes)
            }
        }
    }
}

