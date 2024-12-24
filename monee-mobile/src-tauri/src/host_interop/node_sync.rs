use std::future::Future;

use cream::{context::FromContext, events::dispatcher::Dispatcher};
use monee::{
    nodes::hosts::{
        application::save_host_dir::SetHostBinding,
        domain::{host::host_binding::HostBinding, sync::changes_record::ChangesRecord},
    },
    prelude::AppContext,
};
use monee_core::{ActorId, CurrencyId, WalletId};
use tauri::async_runtime::Sender;
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
        events::domain::event_added::EventAdded, wallets::domain::wallet_created::WalletCreated,
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
}

pub enum DataChanged {
    Currency(CurrencyId),
    Actor(ActorId),
    Wallet(WalletId),
    Event,
}

#[derive(Clone)]
pub struct DataChangedPort(Sender<DataChanged>);

impl DataChangedPort {
    pub async fn send(&self, value: DataChanged) {
        self.0.send(value).await.expect("Failed to send");
    }
}

pub fn setup(ctx: AppContext, host_context: HostContext) -> (DataChangedPort, HostSync) {
    let (changes_tx, changes_rx) = tauri::async_runtime::channel(1);
    let (host_sync, binding_rx, confirmer_tx) = HostSync::create();

    tauri::async_runtime::spawn(listen(
        changes_rx,
        binding_rx,
        confirmer_tx,
        ctx,
        host_context,
    ));
    (DataChangedPort(changes_tx), host_sync)
}

async fn listen(
    mut changes_rx: mpsc::Receiver<DataChanged>,
    mut binding_rx: mpsc::Receiver<Option<HostBinding>>,
    confirmer_tx: watch::Sender<Result<(), InternalError>>,
    ctx: AppContext,
    host_context: HostContext,
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
                        // Should save too?
                        DataChanged::Event => {}
                    }

                    Some(Ok((Ok(()), SyncOrder::SaveChanges)))
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

                            Some(Ok((result, SyncOrder::PullHost)))
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

        let Ok((result, sync_order)) = result else {
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

        let order = match sync_order {
            SyncOrder::SaveChanges => {
                let sync = async {
                    let service: HostCon = host_context.create(host_con);
                    let sync_guide = service.get_guide().await?;

                    let get_service: monee::nodes::sync::application::get_node_changes::GetNodeChanges =
                        ctx.provide();
                    let node_changes = get_service.run(sync_guide, &changes).await?;

                    service.sync_to_host(&node_changes).await?;

                    sync_from_host(&ctx, &host_context, &host_con).await
                };
                do_sync(&ctx, confirmer_tx.clone(), sync).await
            }
            SyncOrder::PullHost => {
                let sync = async { sync_from_host(&ctx, &host_context, &host_con).await };
                do_sync(&ctx, confirmer_tx.clone(), sync).await
            }
        };

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

async fn do_sync(
    ctx: &AppContext,
    confirmer_tx: watch::Sender<Result<(), InternalError>>,
    sync: impl Future<Output = Result<(), AppError<ConnectError>>>,
) -> ChangesOrder {
    let (result, order) = match sync.await.catch_infra(ctx) {
        Ok(Ok(())) => (Ok(()), ChangesOrder::Clear),
        Ok(Err(_)) => {
            eprintln!("WARNING: Failed to connect host, skipping sync");
            (Ok(()), ChangesOrder::Preserve)
        }
        Err(e) => (Err(e), ChangesOrder::Preserve),
    };

    confirmer_tx.send(result).unwrap();
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
