pub mod application {
    pub mod init_self {
        use monee_types::apps::app_id::AppId;

        use crate::{iprelude::*, prelude::*, self_config::domain::repository::Repository};

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct InitSelf {
            repository: Box<dyn Repository>,
        }

        impl InitSelf {
            pub async fn run(&self) -> Result<AppId, InfrastructureError> {
                let id = self.repository.get_self().await?;
                if let Some(id) = id {
                    return Ok(id);
                }

                let id = Default::default();
                self.repository.save_app_id(id).await?;
                Ok(id)
            }
        }
    }
}

