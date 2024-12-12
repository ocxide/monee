pub mod get_events {
    use cream::context::FromContext;

    use crate::{
        reports::events::domain::{event::Event, repository::Repository},
        shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct GetEvents {
        repository: Box<dyn Repository>,
    }

    impl GetEvents {
        pub async fn run(&self) -> Result<Vec<Event>, InfrastructureError> {
            self.repository.get_all().await
        }
    }
}

