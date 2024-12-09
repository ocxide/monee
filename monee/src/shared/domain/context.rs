use cream::context::{
    events_context::{EventsContext, EventsContextBuilder},
    Context, CreamContext, FromContext,
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

pub async fn setup() -> Result<AppContext, InfrastructureError> {
    let db = crate::shared::infrastructure::database::connect().await?;

    let cream = CreamContext::default();
    let mut router = cream::events::router::Router::default();
    // Add event handlers
    router.add::<crate::backoffice::snapshot::application::on_wallet_created::OnWalletCreated>();

    let (events_ctx, setup) = EventsContextBuilder::default().build(&cream);

    let ctx = AppContext {
        events_ctx,
        cream,
        db: DbContext(db),
    };

    setup.setup(router, ctx.clone());

    Ok(ctx)
}

impl FromContext<DbContext> for crate::shared::infrastructure::database::Connection {
    fn from_context(ctx: &DbContext) -> Self {
        ctx.0.clone()
    }
}

mod extends {
    use cream::context::{events_context::EventsContext, ContextExtend, CreamContext};

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
        context::{events_context::EventsContext, pub_provide, CreamContext},
        event_bus::EventBusPort,
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
        client::{
            domain::repository::Repository as ClientRepository,
            infrastructure::repository::SurrealRepository as ClientSurrealRepository,
        },
        sync::{
            domain::repository::Repository as SyncRepository,
            infrastructure::repository::SurrealRepository as SyncSurrealRepository,
        },
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

        ClientRepository: ClientSurrealRepository,
        SyncRepository: SyncSurrealRepository
    }}

    impl cream::context::FromContext<super::AppContext>
        for Box<dyn crate::shared::domain::logging::LogRepository>
    {
        fn from_context(_ctx: &super::AppContext) -> Self {
            Box::new(crate::shared::infrastructure::logging::FileLogRepository)
        }
    }
}
