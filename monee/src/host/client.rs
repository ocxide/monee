pub mod domain {
    pub mod client {
        use super::client_name::ClientName;

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct Client {
            pub name: Option<ClientName>,
        }
    }

    pub mod client_name {
        use crate::shared::domain::alias::Alias;

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct ClientName(Alias);
    }

    pub mod client_id {
        use std::fmt::Display;

        use idn::IdN;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Deserialize, Serialize, Default, Clone, Copy)]
        pub struct ClientId(IdN<4>);

        impl Display for ClientId {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    }

    pub mod repository {
        use crate::prelude::InfrastructureError;

        use super::{client::Client, client_id::ClientId};

        #[async_trait::async_trait]
        pub trait Repository: 'static + Send + Sync {
            async fn save(&self, id: ClientId, client: Client) -> Result<(), InfrastructureError>;

            async fn exists(&self, id: ClientId) -> Result<bool, InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod register_one {
        use crate::host::client::domain::{client::Client, repository::Repository};
        pub use crate::iprelude::*;
        pub use crate::prelude::*;

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct RegisterOne {
            repository: Box<dyn Repository>,
        }

        impl RegisterOne {
            pub async fn run(&self, client: Client) -> Result<(), InfrastructureError> {
                self.repository.save(Default::default(), client).await
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
}

pub mod infrastructure {
    pub mod repository {
        use crate::{
            host::client::domain::{client::Client, client_id::ClientId, repository::Repository},
            shared::{domain::context::DbContext, infrastructure::database::Entity},
        };
        pub use crate::{iprelude::*, prelude::*};

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: ClientId, client: Client) -> Result<(), InfrastructureError> {
                self.0
                    .query("CREATE type::thing('client', ) CONTENT ")
                    .bind(("id", id))
                    .bind(("data", client))
                    .await?;

                Ok(())
            }

            async fn exists(&self, id: ClientId) -> Result<bool, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT id FROM ONLY client WHERE id =  LIMIT 1")
                    .bind(("id", id))
                    .await?
                    .check()?;

                let entity: Option<Entity<ClientId, ()>> = response.take(0)?;
                Ok(entity.is_some())
            }
        }
    }
}
