pub mod register_one {
    use crate::host::client::domain::client_id::ClientId;
    use crate::host::client::domain::{client::Client, repository::Repository};
    pub use crate::iprelude::*;
    pub use crate::prelude::*;

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct RegisterOne {
        repository: Box<dyn Repository>,
    }

    impl RegisterOne {
        pub async fn run(&self, client: Client) -> Result<ClientId, InfrastructureError> {
            let id = Default::default();
            self.repository.save(id, client).await.map(|_| id)
        }
    }
}

pub mod exists {
    use crate::host::client::domain::{client_id::ClientId, repository::Repository};
    pub use crate::iprelude::*;
    pub use crate::prelude::*;

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct Exists {
        repository: Box<dyn Repository>,
    }

    impl Exists {
        pub async fn run(&self, id: ClientId) -> Result<bool, InfrastructureError> {
            self.repository.exists(id).await
        }
    }
}

