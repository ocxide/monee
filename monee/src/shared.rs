pub mod application {
    pub mod logging {
        use cream::context::ContextProvide;

        use crate::shared::{
            domain::{context::AppContext, logging::LogRepository},
            infrastructure::errors::InfrastructureError,
        };

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct LogService {
            repository: Box<dyn LogRepository>,
        }

        impl LogService {
            pub fn error(&self, err: InfrastructureError) {
                let result = self.repository.log(format_args!("{:?}", err));
                if let Err(e) = result {
                    println!("error logging error: {:?}", e);
                }
            }
        }
    }
}

pub mod domain {
    pub mod logging {
        use crate::shared::infrastructure::errors::InfrastructureError;

        pub trait LogRepository {
            fn log(&self, message: std::fmt::Arguments) -> Result<(), InfrastructureError>;
        }
    }

    pub mod errors {
        use crate::shared::infrastructure::errors::IntoAppResult;

        #[derive(PartialEq, Eq)]
        pub enum UniqueSaveStatus {
            Created,
            AlreadyExists,
        }

        impl UniqueSaveStatus {
            pub fn is_ok(&self) -> bool {
                matches!(self, UniqueSaveStatus::Created)
            }
        }

        pub enum UniqueUpdateStatus {
            Updated,
            NotFound,
            Conflict,
        }

        pub enum UniqueSaveError {
            AlreadyExists,
        }

        pub(crate) trait IntoDomainResult<T, E> {
            fn into_domain_result(self) -> Result<T, E>;
        }

        impl
            IntoDomainResult<
                UniqueSaveStatus,
                crate::shared::infrastructure::errors::InfrastructureError,
            > for Result<surrealdb::Response, surrealdb::Error>
        {
            fn into_domain_result(
                self,
            ) -> Result<UniqueSaveStatus, crate::shared::infrastructure::errors::InfrastructureError>
            {
                match self {
                    Ok(_) => Ok(UniqueSaveStatus::Created),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Ok(UniqueSaveStatus::AlreadyExists),
                    Err(e) => Err(e.into()),
                }
            }
        }

        impl IntoAppResult<UniqueSaveError> for Result<surrealdb::Response, surrealdb::Error> {
            fn into_app_result(
                self,
            ) -> Result<(), crate::shared::infrastructure::errors::AppError<UniqueSaveError>>
            {
                match self {
                    Ok(_) => Ok(()),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(crate::shared::infrastructure::errors::AppError::App(
                        UniqueSaveError::AlreadyExists,
                    )),
                    Err(e) => Err(
                        crate::shared::infrastructure::errors::AppError::Infrastructure(e.into()),
                    ),
                }
            }
        }
    }

    pub mod context {
        use cream::context::{ContextProvide, CreamContext};

        use crate::shared::infrastructure::errors::InfrastructureError;

        #[derive(Clone)]
        pub struct AppContext {
            cream: CreamContext,
            db: DbContext,
        }

        impl AppContext {
            pub fn provide<S>(&self) -> S
            where
                Self: ContextProvide<S>,
            {
                <Self as ContextProvide<S>>::provide(self)
            }
        }

        #[derive(Clone)]
        pub struct DbContext(crate::shared::infrastructure::database::Connection);

        #[cfg(all(test, feature = "db_test"))]
        impl DbContext {
            pub(crate) fn new(
                connection: crate::shared::infrastructure::database::Connection,
            ) -> Self {
                Self(connection)
            }
        }

        impl ContextProvide<crate::shared::infrastructure::database::Connection> for DbContext {
            fn provide(&self) -> crate::shared::infrastructure::database::Connection {
                self.0.clone()
            }
        }

        trait ContextExtend<C> {
            fn provide_context(&self) -> &C;
        }

        impl ContextExtend<DbContext> for AppContext {
            fn provide_context(&self) -> &DbContext {
                &self.db
            }
        }

        impl<S> ContextProvide<S> for AppContext
        where
            CreamContext: ContextProvide<S>,
        {
            fn provide(&self) -> S {
                self.cream.provide()
            }
        }

        pub async fn setup(
        ) -> Result<(AppContext, impl std::future::Future<Output = ()>), InfrastructureError>
        {
            let db = crate::shared::infrastructure::database::connect().await?;

            let router = cream::events::router::Router::default();
            // Add event handlers

            let (port, socket) = cream::router_bus::create_channel();

            let ctx = AppContext {
                cream: CreamContext::new(port),
                db: DbContext(db),
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

        mod provide_maps {
            use crate::backoffice::{
                actors::{
                    domain::repository::Repository as ActorsRepository,
                    infrastructure::repository::SurrealRepository as ActorsSurrealRepository,
                },
                currencies::{
                    domain::repository::Repository as CurrenciesRepository,
                    infrastructure::repository::SurrealRepository as CurrenciesSurrealRepository,
                },
                events::{
                    domain::repository::Repository as EventsRepository,
                    infrastructure::repository::SurrealRepository as EventsSurrealRepository,
                },
                item_tags::{
                    domain::repository::Repository as ItemTagsRepository,
                    infrastructure::repository::SurrealRepository as ItemTagsSurrealRepository,
                },
                snapshot::{
                    domain::repository::SnapshotRepository,
                    infrastructure::snapshot_repository::SnapshotSurrealRepository,
                },
                wallets::{
                    domain::repository::Repository as WalletsRepository,
                    infrastructure::repository::SurrealRepository as WalletsSurrealRepository,
                },
            };

            use super::DbContext;

            macro_rules! provide_map(($service: path; $real_service: path, $ctx: ident) => {
                impl cream::context::ContextProvide<Box<dyn $service>> for super::AppContext {
                    fn provide(&self) -> Box<dyn $service> {
                        let ctx = <Self as super::ContextExtend<$ctx>>::provide_context(self);
                        let real_service: $real_service = ctx.provide();
                        Box::new(real_service)
                    }
                }
            });

            provide_map!(SnapshotRepository; SnapshotSurrealRepository, DbContext);
            provide_map!(WalletsRepository; WalletsSurrealRepository, DbContext);
            provide_map!(ActorsRepository; ActorsSurrealRepository, DbContext);
            provide_map!(CurrenciesRepository; CurrenciesSurrealRepository, DbContext);
            provide_map!(ItemTagsRepository; ItemTagsSurrealRepository, DbContext);
            provide_map!(EventsRepository; EventsSurrealRepository, DbContext);
            provide_map!(
                crate::reports::snapshot::domain::repository::Repository;
                crate::reports::snapshot::infrastructure::repository::SurrealRepository,
                DbContext
            );
            provide_map!(
                crate::reports::events::domain::repository::Repository;
                crate::reports::events::infrastructure::repository::SurrealRepository,
                DbContext
            );

            impl
                cream::context::ContextProvide<
                    Box<dyn crate::shared::domain::logging::LogRepository>,
                > for super::AppContext
            {
                fn provide(&self) -> Box<dyn crate::shared::domain::logging::LogRepository> {
                    Box::new(crate::shared::infrastructure::logging::FileLogRepository)
                }
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
