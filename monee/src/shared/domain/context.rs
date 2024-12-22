use std::path::PathBuf;

use cream::{
    context::{Context, CreamContext, FromContext},
    events::{
        context::{EventsContext, EventsContextBuilder, EventsContextSetup},
        dispatch_listener::DispatchListener,
    },
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
impl Context for DbContext {}

#[cfg(all(test, feature = "db_test"))]
impl DbContext {
    pub(crate) fn new(connection: crate::shared::infrastructure::database::Connection) -> Self {
        Self(connection)
    }
}

pub struct AppContextBuilder {
    pub base_dir: PathBuf,
}

impl AppContextBuilder {
    pub async fn build(self) -> Result<AppContextSetup, InfrastructureError> {
        #[cfg(any(feature = "embedded", feature = "remote"))]
        let db = crate::shared::infrastructure::database::connect(self.base_dir).await?;
        #[cfg(feature = "db_test")]
        let db = crate::shared::infrastructure::database::connect().await?;

        let cream = CreamContext::default();
        let (events_ctx, setup) = cream.provide::<EventsContextBuilder>().build();

        let ctx = AppContext {
            events_ctx,
            cream,
            db: DbContext(db),
        };

        Ok(AppContextSetup {
            ctx,
            events_setup: setup,
        })
    }
}

pub struct AppContextSetup {
    ctx: AppContext,
    events_setup: EventsContextSetup,
}

impl AppContextSetup {
    pub fn setup(self) -> AppContext {
        let mut dispatcher = cream::events::dispatcher::Dispatcher::<AppContext>::default();
        // Add event handlers
        dispatcher
            .add::<crate::backoffice::snapshot::application::on_wallet_created::OnWalletCreated>();

        self.events_setup
            .setup::<DispatchListener<_>>((self.ctx.clone(), dispatcher));

        self.ctx
    }
}

pub async fn setup() -> Result<AppContext, InfrastructureError> {
    #[cfg(feature = "embedded")]
    let base_dir = crate::shared::infrastructure::filesystem::create_local_path();
    #[cfg(not(feature = "embedded"))]
    let base_dir = PathBuf::default();

    let setup = AppContextBuilder { base_dir }.build().await?;
    Ok(setup.setup())
}

impl FromContext<DbContext> for crate::shared::infrastructure::database::Connection {
    fn from_context(ctx: &DbContext) -> Self {
        ctx.0.clone()
    }
}

mod extends {
    use cream::{
        context::{ContextExtend, CreamContext},
        events::context::EventsContext,
    };

    use super::{AppContext, DbContext};

    impl ContextExtend<DbContext> for AppContext {
        fn provide_ctx(&self) -> &DbContext {
            &self.db
        }
    }

    impl ContextExtend<CreamContext> for AppContext {
        fn provide_ctx(&self) -> &CreamContext {
            &self.cream
        }
    }

    impl ContextExtend<EventsContext> for AppContext {
        fn provide_ctx(&self) -> &EventsContext {
            &self.events_ctx
        }
    }
}

mod provides_export {
    use cream::{
        context::{pub_provide, CreamContext},
        events::{bus::EventBusPort, context::EventsContext},
        tasks::{Shutdown, Tasks},
    };

    use super::AppContext;

    pub_provide!(AppContext : CreamContext { Tasks, Shutdown });
    pub_provide!(AppContext : EventsContext { EventBusPort });
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

    use crate::host::{
        nodes::{
            domain::repository::Repository as ClientRepository,
            infrastructure::repository::SurrealRepository as ClientSurrealRepository,
        },
        sync::{
            domain::repository::Repository as SyncRepository,
            infrastructure::repository::SurrealRepository as SyncSurrealRepository,
        },
    };

    use crate::nodes::{
        hosts::{
            domain::repository::Repository as HostsRepository,
            infrastructure::repository::SurrealRepository as HostsSurrealRepository,
        },
        sync::{
            domain::repository::Repository as NodeSyncRepository,
            infrastructure::repository::SurrealRepository as NodeSyncSurrealRepository,
        },
    };

    use crate::self_config::{
        domain::repository::Repository as SelfConfigRepository,
        infrastructure::repository::SurrealRepository as SelfConfigSurrealRepository,
    };

    use super::{AppContext, DbContext};

    macro_rules! provide_map (($ctx: path { $($service: path: $real_service: path),* $(,)* }) => {
        $(
        impl cream::context::FromContext<AppContext> for Box<dyn $service> {
            fn from_context(app_ctx: &AppContext) -> Self {
                use cream::context::{ContextExtend, Context};
                let ctx: &$ctx = app_ctx.provide_ctx();
                let real_service: $real_service = ctx.provide();
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

        SelfConfigRepository: SelfConfigSurrealRepository,

        ClientRepository: ClientSurrealRepository,
        SyncRepository: SyncSurrealRepository,

        HostsRepository: HostsSurrealRepository,
        NodeSyncRepository: NodeSyncSurrealRepository
    }}

    impl cream::context::FromContext<super::AppContext>
        for Box<dyn crate::shared::domain::logging::LogRepository>
    {
        fn from_context(_ctx: &super::AppContext) -> Self {
            Box::new(crate::shared::infrastructure::logging::FileLogRepository)
        }
    }
}
