pub mod errors {
    #[derive(Debug, thiserror::Error)]
    #[error("infrastructure error: {0}")]
    pub struct InfrastructureError(Box<dyn std::error::Error>);

    impl InfrastructureError {
        pub fn new<E>(error: E) -> Self
        where
            E: Into<Box<dyn std::error::Error>>,
        {
            Self(error.into())
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum SnapshotOptError {
        #[error(transparent)]
        Infrastructure(#[from] InfrastructureError),

        #[error(transparent)]
        SnapshotApply(#[from] monee_core::Error),

        #[error(transparent)]
        Write(#[from] std::io::Error),

        #[error(transparent)]
        Read(#[from] crate::snapshot_io::ReadError),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum SnapshotWriteError {
        #[error(transparent)]
        Infrastructure(#[from] InfrastructureError),

        #[error(transparent)]
        SnapshotApply(#[from] monee_core::Error),

        #[error(transparent)]
        Write(#[from] std::io::Error),
    }

    impl From<SnapshotWriteError> for SnapshotOptError {
        fn from(value: SnapshotWriteError) -> Self {
            match value {
                SnapshotWriteError::Infrastructure(error) => Self::Infrastructure(error),
                SnapshotWriteError::SnapshotApply(error) => Self::SnapshotApply(error),
                SnapshotWriteError::Write(error) => Self::Write(error),
            }
        }
    }

    impl From<SnapshotReadError> for SnapshotOptError {
        fn from(value: SnapshotReadError) -> Self {
            match value {
                SnapshotReadError::Infrastructure(error) => Self::Infrastructure(error),
                SnapshotReadError::SnapshotApply(error) => Self::SnapshotApply(error),
                SnapshotReadError::Read(error) => Self::Read(error),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum SnapshotReadError {
        #[error(transparent)]
        Infrastructure(#[from] InfrastructureError),

        #[error(transparent)]
        SnapshotApply(#[from] monee_core::Error),

        #[error(transparent)]
        Read(#[from] crate::snapshot_io::ReadError),
    }
}

pub mod application {
    pub mod cannonical_context {
        use cream::context::{ContextExtend, CreamContext};

        use crate::shared::{domain::context::AppContext, errors::InfrastructureError};

        #[derive(Clone)]
        pub struct CannocalContext {
            cream_context: CreamContext,
            database: crate::shared::infrastructure::database::Connection,
        }

        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),
        }

        pub async fn setup(
        ) -> Result<(CannocalContext, impl std::future::Future<Output = ()>), Error> {
            let db = crate::shared::infrastructure::database::connect()
                .await
                .map_err(InfrastructureError::new)?;

            let router = cream::events::router::Router::default();
            // Add event handlers

            let (port, socket) = cream::router_bus::create_channel();

            let ctx = CannocalContext {
                cream_context: CreamContext::new(port),
                database: db,
            };

            let listen = {
                let ctx = ctx.clone();
                async move {
                    cream::router_bus::RouterBus::new(socket, ctx, router)
                        .listen()
                        .await;
                }
            };

            Ok((ctx, listen))
        }

        impl AppContext for CannocalContext {
            fn backoffice_events_repository(
                &self,
            ) -> Box<dyn crate::backoffice::events::domain::repository::Repository> {
                Box::new(
                    crate::backoffice::events::infrastructure::repository::SurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }

            fn backoffice_wallets_repository(
                &self,
            ) -> Box<dyn crate::backoffice::wallets::domain::repository::Repository> {
                Box::new(
                    crate::backoffice::wallets::infrastructure::repository::SurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }

            fn backoffice_actors_repository(
                &self,
            ) -> Box<dyn crate::backoffice::actors::domain::repository::Repository> {
                Box::new(
                    crate::backoffice::actors::infrastructure::repository::SurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }

            fn backoffice_currencies_repository(
                &self,
            ) -> Box<dyn crate::backoffice::currencies::domain::repository::Repository>
            {
                Box::new(
                    crate::backoffice::currencies::infrastructure::repository::SurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }

            fn backoffice_item_tags_repository(
                &self,
            ) -> Box<dyn crate::backoffice::item_tags::domain::repository::Repository> {
                Box::new(
                    crate::backoffice::item_tags::infrastructure::repository::SurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }

            fn backoffice_snapshot_repository(
                &self,
            ) -> Box<dyn crate::backoffice::snapshot::domain::repository::SnapshotRepository>
            {
                Box::new(
                    crate::backoffice::snapshot::infrastructure::snapshot_repository::SnapshotSurrealRepository::new(
                        self.database.clone(),
                    ),
                )
            }
        }

        impl ContextExtend<CreamContext> for CannocalContext {
            fn provide_context(&self) -> &CreamContext {
                &self.cream_context
            }
        }
    }
}

pub mod domain {
    pub mod context {
        use cream::context::{ContextExtend, CreamContext, FromContext};

        pub trait AppContext: ContextExtend<CreamContext> {
            fn backoffice_events_repository(
                &self,
            ) -> Box<dyn crate::backoffice::events::domain::repository::Repository>;

            fn backoffice_wallets_repository(
                &self,
            ) -> Box<dyn crate::backoffice::wallets::domain::repository::Repository>;

            fn backoffice_actors_repository(
                &self,
            ) -> Box<dyn crate::backoffice::actors::domain::repository::Repository>;

            fn backoffice_currencies_repository(
                &self,
            ) -> Box<dyn crate::backoffice::currencies::domain::repository::Repository>;

            fn backoffice_item_tags_repository(
                &self,
            ) -> Box<dyn crate::backoffice::item_tags::domain::repository::Repository>;

            fn backoffice_snapshot_repository(
                &self,
            ) -> Box<dyn crate::backoffice::snapshot::domain::repository::SnapshotRepository>;
        }

        impl<C> FromContext<C> for cream::event_bus::EventBusPort
        where
            C: AppContext,
        {
            fn from_context(ctx: &C) -> Self {
                let cream_context = ctx.provide_context();
                Self::from_context(cream_context)
            }
        }
    }

    pub(crate) mod alias {
        #[derive(Debug, Clone)]
        pub struct Alias(Box<str>);

        impl Alias {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for Alias {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        pub mod from_str {
            use super::Alias;

            #[derive(Debug)]
            pub enum Error {
                Empty,
                Invalid,
            }

            impl std::fmt::Display for Error {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Error::Empty => write!(f, "Alias cannot be emtpy"),
                        Error::Invalid => write!(
                            f,
                            "Alias must only contain 'a-z', 'A-Z', '0-9', '-', '_' characters"
                        ),
                    }
                }
            }

            impl std::error::Error for Error {}

            impl std::str::FromStr for Alias {
                type Err = Error;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    if s.is_empty() {
                        return Err(Error::Empty);
                    }

                    let is_valid = s
                        .chars()
                        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'));

                    if is_valid {
                        Ok(Alias(s.into()))
                    } else {
                        Err(Error::Invalid)
                    }
                }
            }
        }
    }
}

pub mod infrastructure;
