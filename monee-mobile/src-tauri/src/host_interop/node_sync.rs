use std::future::Future;

use cream::{context::FromContext, events::dispatcher::Dispatcher};
use monee::{
    nodes::hosts::{
        application::save_host_dir::SetHostBinding,
        domain::{host::host_binding::HostBinding, sync::changes_record::ChangesRecord},
    },
    prelude::AppContext,
};
use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};
use tauri::{async_runtime::Sender, AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

use crate::{prelude::*, CatchInfra};

use super::{
    host_context::{ConnectError, HostCon, HostContext},
    host_sync::HostSync,
};

#[derive(Clone)]
pub struct NodeSyncContext {
    port: DataChangedPort,
}

impl NodeSyncContext {
    pub fn setup(port: DataChangedPort) -> (Self, Dispatcher<NodeSyncContext>) {
        let ctx = NodeSyncContext { port };
        let mut dispatcher = Dispatcher::default();

        dispatcher.add::<handlers::OnWalletCreated>();
        dispatcher.add::<handlers::OnCurrencyCreated>();
        dispatcher.add::<handlers::OnActorCreated>();
        dispatcher.add::<handlers::OnEventAdded>();
        dispatcher.add::<handlers::OnItemCreated>();

        (ctx, dispatcher)
    }
}

impl Context for NodeSyncContext {}

impl FromContext<NodeSyncContext> for DataChangedPort {
    fn from_context(ctx: &NodeSyncContext) -> Self {
        ctx.port.clone()
    }
}

mod handlers {
    use cream::{
        context::FromContext,
        events::{Error, Handler},
    };
    use monee::backoffice::{
        actors::domain::actor_created::ActorCreated,
        currencies::domain::currency_created::CurrencyCreated,
        events::domain::event_added::EventAdded,
        item_tags::domain::item_tag_created::ItemTagCreated,
        wallets::domain::wallet_created::WalletCreated,
    };

    use super::{DataChangedPort, NodeSyncContext};

    #[derive(FromContext)]
    #[context(NodeSyncContext)]
    pub struct OnWalletCreated {
        port: DataChangedPort,
    }
    impl Handler for OnWalletCreated {
        type Event = WalletCreated;
        async fn handle(self, event: Self::Event) -> Result<(), Error> {
            self.port.send(super::DataChanged::Wallet(event.id)).await;
            Ok(())
        }
    }

    #[derive(FromContext)]
    #[context(NodeSyncContext)]
    pub struct OnCurrencyCreated {
        port: DataChangedPort,
    }
    impl Handler for OnCurrencyCreated {
        type Event = CurrencyCreated;
        async fn handle(self, event: Self::Event) -> Result<(), Error> {
            self.port.send(super::DataChanged::Currency(event.id)).await;
            Ok(())
        }
    }

    #[derive(FromContext)]
    #[context(NodeSyncContext)]
    pub struct OnActorCreated {
        port: DataChangedPort,
    }

    impl Handler for OnActorCreated {
        type Event = ActorCreated;
        async fn handle(self, event: Self::Event) -> Result<(), Error> {
            self.port.send(super::DataChanged::Actor(event.id)).await;
            Ok(())
        }
    }

    #[derive(FromContext)]
    #[context(NodeSyncContext)]
    pub struct OnEventAdded {
        port: DataChangedPort,
    }

    impl Handler for OnEventAdded {
        type Event = EventAdded;
        async fn handle(self, _: Self::Event) -> Result<(), Error> {
            self.port.send(super::DataChanged::Event).await;
            Ok(())
        }
    }

    #[derive(FromContext)]
    #[context(NodeSyncContext)]
    pub struct OnItemCreated {
        port: DataChangedPort,
    }

    impl Handler for OnItemCreated {
        type Event = ItemTagCreated;
        async fn handle(self, event: Self::Event) -> Result<(), Error> {
            self.port.send(super::DataChanged::Item(event.id)).await;
            Ok(())
        }
    }
}

pub enum DataChanged {
    Currency(CurrencyId),
    Actor(ActorId),
    Wallet(WalletId),
    Item(ItemTagId),
    Event,
}

#[derive(Clone)]
pub struct DataChangedPort(Sender<DataChanged>);

impl DataChangedPort {
    pub async fn send(&self, value: DataChanged) {
        self.0.send(value).await.expect("Failed to send");
    }
}

pub fn setup(
    ctx: AppContext,
    host_context: HostContext,
    tauri_app: AppHandle,
) -> (DataChangedPort, HostSync) {
    let (changes_tx, changes_rx) = tauri::async_runtime::channel(1);
    let (host_sync, binding_rx, confirmer_tx) = HostSync::create();

    tauri::async_runtime::spawn(listen(
        changes_rx,
        binding_rx,
        confirmer_tx,
        ctx,
        host_context,
        tauri_app,
    ));
    (DataChangedPort(changes_tx), host_sync)
}

async fn listen(
    mut changes_rx: mpsc::Receiver<DataChanged>,
    mut binding_rx: mpsc::Receiver<Option<HostBinding>>,
    confirmer_tx: watch::Sender<Result<(), InternalError>>,
    ctx: AppContext,
    host_context: HostContext,
    tauri_app: AppHandle,
) {
    let changes_getter: monee::nodes::changes::application::load::Load = ctx.provide();

    let mut changes = changes_getter.run().await.expect("to load initial changes");
    let mut host_con = None;

    enum SyncOrder {
        SaveChanges,
        PullHost,
    }

    loop {
        let result = tokio::select! {
            data_changed = changes_rx.recv() => {
                if let Some(data_changed) = data_changed {
                    match data_changed {
                        DataChanged::Currency(id) => changes.currencies.push(id),
                        DataChanged::Actor(id) => changes.actors.push(id),
                        DataChanged::Wallet(id) => changes.wallets.push(id),
                        DataChanged::Item(id) => changes.items.push(id),
                        // Should save too?
                        DataChanged::Event => {}
                    }

                    Some(Ok(Ok(())))
                }
                else {
                    None
                }
            },

            host_changed = binding_rx.recv() => {
                if let Some(host_changed) = host_changed {
                    host_con = host_changed;

                    match host_con.as_ref() {
                        Some(host_con) => {
                            let host_set: SetHostBinding = ctx.provide();
                            let result = host_set.run(host_con).await.catch_infra(&ctx);

                            Some(Ok(result))
                        }
                        None => Some(Err(())),
                    }
                } else { None }
            }
        };

        let Some(result) = result else {
            eprintln!("WARNING: sync channel closed");
            break;
        };

        let Ok(result) = result else {
            eprint!("WARNING: skipping sync: {}", file!());
            continue;
        };

        let Some(host_con) = &host_con else {
            eprintln!("WARNING: host binding not set");
            continue;
        };

        if let Err(e) = result {
            confirmer_tx.send(Err(e)).unwrap();
            continue;
        }

        let sync = async {
            let service: HostCon = host_context.create(host_con);
            let sync_guide = service.get_guide().await?;

            let get_service: monee::nodes::sync::application::get_node_changes::GetNodeChanges =
                ctx.provide();
            let node_changes = get_service.run(sync_guide, &changes).await?;

            dbg!("SYNC TO HOST");
            service.sync_to_host(&node_changes).await?;
            println!("SYNCED TO HOST");

            sync_from_host(&ctx, &host_context, &host_con).await
        };
        let order = do_sync(&ctx, confirmer_tx.clone(), sync, &tauri_app).await;

        if let ChangesOrder::Clear = order {
            changes = ChangesRecord::default();
        }

        let changes_saver: monee::nodes::changes::application::save::Save = ctx.provide();
        if let Err(e) = changes_saver.run(&changes).await {
            eprintln!("WARNING: failed to save changes: {}", e);
        };
    }
}

enum ChangesOrder {
    Preserve,
    Clear,
}

#[derive(Debug, Clone, serde::Serialize)]
enum HostStatus {
    Online,
    Offline,
}

async fn do_sync(
    ctx: &AppContext,
    confirmer_tx: watch::Sender<Result<(), InternalError>>,
    sync: impl Future<Output = Result<(), AppError<ConnectError>>>,
    tauri_app: &AppHandle,
) -> ChangesOrder {
    let (result, order, host_status) = match sync.await.catch_infra(ctx) {
        Ok(Ok(())) => (Ok(()), ChangesOrder::Clear, HostStatus::Online),
        Ok(Err(_)) => {
            eprintln!("WARNING: Failed to connect host, skipping sync");
            (Ok(()), ChangesOrder::Preserve, HostStatus::Offline)
        }
        Err(e) => (Err(e), ChangesOrder::Preserve, HostStatus::Offline),
    };

    confirmer_tx.send(result).unwrap();
    tauri_app.emit("host_status_change", host_status).unwrap();

    order
}

async fn sync_from_host(
    ctx: &AppContext,
    host_context: &HostContext,
    host_binding: &HostBinding,
) -> Result<(), AppError<ConnectError>> {
    let service: HostCon = host_context.create(host_binding);
    let host_state = service.get_host_state().await?;

    let service: monee::nodes::sync::application::rewrite_system::RewriteSystem = ctx.provide();
    let result = match service.run(host_state).await {
        Ok(_) => Ok(()),
        Err(AppError::Infrastructure(e)) => return Err(e.into()),
        Err(AppError::App(e)) => Err(e),
    };
    if let Err(e) = result {
        eprintln!("WARNING: Failed to overwrite system: error: {e:?}");
    }

    Ok(())
}
