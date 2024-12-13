pub mod register_one {
    use monee_types::apps::app_id::AppId;
    use monee_types::apps::app_manifest::AppManifest;

    use crate::host::client::domain::repository::Repository;
    pub use crate::iprelude::*;
    pub use crate::prelude::*;

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct RegisterOne {
        repository: Box<dyn Repository>,
    }

    impl RegisterOne {
        pub async fn run(&self, app: AppManifest) -> Result<AppId, InfrastructureError> {
            let id = Default::default();
            self.repository.save(id, app).await?;
            Ok(id)
        }
    }
}

pub mod exists {
    use monee_types::apps::app_id::AppId;

    use crate::host::client::domain::repository::Repository;
    pub use crate::iprelude::*;
    pub use crate::prelude::*;

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct Exists {
        repository: Box<dyn Repository>,
    }

    impl Exists {
        pub async fn run(&self, id: AppId) -> Result<bool, InfrastructureError> {
            self.repository.exists(id).await
        }
    }
}
