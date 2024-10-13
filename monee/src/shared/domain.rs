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
        ) -> Result<(), crate::shared::infrastructure::errors::AppError<UniqueSaveError>> {
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
                Err(e) => {
                    Err(crate::shared::infrastructure::errors::AppError::Infrastructure(e.into()))
                }
            }
        }
    }
}

pub mod context {
    use cream::context::{
        events_context::{EventsContext, EventsContextBuilder},
        Context, ContextProvide, CreamContext,
    };

    use crate::shared::infrastructure::errors::InfrastructureError;

    #[derive(Clone)]
    pub struct AppContext {
        cream: CreamContext,
        events_ctx: EventsContext,
        db: DbContext,
    }

    impl Context for AppContext {}

    #[derive(Clone)]
    pub struct DbContext(crate::shared::infrastructure::database::Connection);

    #[cfg(all(test, feature = "db_test"))]
    impl DbContext {
        pub(crate) fn new(connection: crate::shared::infrastructure::database::Connection) -> Self {
            Self(connection)
        }
    }

    pub async fn setup() -> Result<AppContext, InfrastructureError> {
        let db = crate::shared::infrastructure::database::connect().await?;

        let cream = CreamContext::default();
        let router = cream::events::router::Router::default();
        // Add event handlers
        let (events_ctx, setup) = EventsContextBuilder::default().build(&cream);

        let ctx = AppContext {
            events_ctx,
            cream,
            db: DbContext(db),
        };

        setup.setup(router, ctx.clone());

        Ok(ctx)
    }

    impl ContextProvide<crate::shared::infrastructure::database::Connection> for DbContext {
        fn ctx_provide(&self) -> crate::shared::infrastructure::database::Connection {
            self.0.clone()
        }
    }

    mod extends {
        use cream::context::{events_context::EventsContext, CreamContext};

        use super::{AppContext, DbContext};

        pub trait ContextExtend<C> {
            fn provide_context(&self) -> &C;
        }

        impl ContextExtend<DbContext> for AppContext {
            fn provide_context(&self) -> &DbContext {
                &self.db
            }
        }

        impl ContextExtend<CreamContext> for AppContext {
            fn provide_context(&self) -> &CreamContext {
                &self.cream
            }
        }

        impl ContextExtend<EventsContext> for AppContext {
            fn provide_context(&self) -> &EventsContext {
                &self.events_ctx
            }
        }
    }

    mod provides_export {
        use cream::{
            context::{events_context::EventsContext, ContextProvide, CreamContext},
            event_bus::EventBusPort,
            tasks::Tasks,
        };

        use super::{extends::ContextExtend, AppContext};
        macro_rules! pub_provide (($provider: path { $($service: path),* }) => {
                $(
                impl ContextProvide<$service> for AppContext {
                    fn ctx_provide(&self) -> $service {
                        let ctx =
                            <super::AppContext as ContextExtend<$provider>>::provide_context(self);
                        ctx.ctx_provide()
                    }
                }
                )*
            };
        );

        pub_provide!(CreamContext { Tasks });
        pub_provide!(EventsContext { EventBusPort });
    }

    mod provides_config {
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

        use super::{extends::ContextExtend, AppContext, DbContext};

        macro_rules! provide_map (($ctx: path { $($service: path: $real_service: path),* $(,)* }) => {
            $(
            impl cream::context::ContextProvide<Box<dyn $service>> for AppContext {
                fn ctx_provide(&self) -> Box<dyn $service> {
                    let ctx = <Self as ContextExtend<$ctx>>::provide_context(self);
                    let real_service: $real_service = ctx.ctx_provide();
                    Box::new(real_service)
                }
            }
            )*
        });

        provide_map! {DbContext {
            SnapshotRepository: SnapshotSurrealRepository,
            WalletsRepository: WalletsSurrealRepository,
            ActorsRepository: ActorsSurrealRepository,
            CurrenciesRepository: CurrenciesSurrealRepository,
            ItemTagsRepository: ItemTagsSurrealRepository,
            EventsRepository: EventsSurrealRepository,
            crate::reports::snapshot::domain::repository::Repository: crate::reports::snapshot::infrastructure::repository::SurrealRepository,
            crate::reports::events::domain::repository::Repository: crate::reports::events::infrastructure::repository::SurrealRepository,
        }}

        impl cream::context::ContextProvide<Box<dyn crate::shared::domain::logging::LogRepository>>
            for super::AppContext
        {
            fn ctx_provide(&self) -> Box<dyn crate::shared::domain::logging::LogRepository> {
                Box::new(crate::shared::infrastructure::logging::FileLogRepository)
            }
        }
    }
}

pub(crate) mod alias {
    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
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
