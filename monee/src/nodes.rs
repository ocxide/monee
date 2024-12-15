pub mod hosts;
pub mod sync;
pub mod config {
    pub mod application {
        pub mod save_self {
            use monee_types::apps::app_id::AppId;

            use crate::iprelude::*;
            use crate::prelude::*;
            use crate::self_config::domain::repository::Repository;

            #[derive(FromContext)]
            #[context(AppContext)]
            pub struct SaveAppId {
                repository: Box<dyn Repository>,
            }

            impl SaveAppId {
                pub async fn run(&self, id: AppId) -> Result<(), InfrastructureError> {
                    self.repository.save_app_id(id).await
                }
            }
        }
    }
}
