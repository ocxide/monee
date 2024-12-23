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

use crate::prelude::*;

use super::{
    host_context::{HostCon, HostContext},
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
            self.port.send(super::DataChanged::Wallet(event.id));
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
            self.port.send(super::DataChanged::Currency(event.id));
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
            self.port.send(super::DataChanged::Actor(event.id));
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
            self.port.send(super::DataChanged::Event);
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
    let mut changes = ChangesRecord::default();
    let mut host_con = None;

    const MSG: &str = "WARNING: changes not sent due no host connection";

    loop {
        tokio::select! {
            data_changed = changes_rx.recv() => {
                if let Some(data_changed) = data_changed {
                    if let Some(host_binding) = host_con.as_ref() {
                        let result = on_changes(&ctx, &host_context, &mut changes, host_binding, data_changed).await ;
                        confirmer_tx.send(result).unwrap();
                    }
                    else {
                       eprintln!("ERROR SYNC SYSTEM: {}", MSG);
                    }
                }
            },

            host_changed = binding_rx.recv() => {
                if let Some(host_changed) = host_changed {
                    host_con = host_changed;
                    if let Some(host_binding) = host_con.as_ref() {
                        let result = on_host_set(&ctx, &host_context, &mut changes, host_binding).await;
                        confirmer_tx.send(result).unwrap();
                    }
                    else {
                       eprintln!("ERROR SYNC SYSTEM: {}", MSG);
                    }
                }
            }
        };
    }
}

async fn on_changes(
    ctx: &AppContext,
    host_context: &HostContext,
    changes_record: &mut ChangesRecord,
    host_binding: &HostBinding,
    change: DataChanged,
) -> Result<(), InternalError> {
    match change {
        DataChanged::Currency(id) => changes_record.currencies.push(id),
        DataChanged::Actor(id) => changes_record.actors.push(id),
        DataChanged::Wallet(id) => changes_record.wallets.push(id),
        // Should save too?
        DataChanged::Event => {}
    }

    let service: HostCon = host_context.create(host_binding);
    let sync_guide = service.get_guide().await.catch_infra(ctx)?;

    let get_service: monee::nodes::sync::application::get_node_changes::GetNodeChanges =
        ctx.provide();
    let node_changes = get_service
        .run(sync_guide, changes_record)
        .await
        .catch_infra(ctx)?;

    service.sync_to_host(&node_changes).await.catch_infra(ctx)?;

    sync_from_host(ctx, host_context, host_binding, changes_record).await
}

async fn on_host_set(
    ctx: &AppContext,
    host_context: &HostContext,
    changes_record: &mut ChangesRecord,
    host_binding: &HostBinding,
) -> Result<(), InternalError> {
    let host_set: SetHostBinding = ctx.provide();
    host_set.run(host_binding).await.catch_infra(ctx)?;

    sync_from_host(ctx, host_context, host_binding, changes_record).await
}

async fn sync_from_host(
    ctx: &AppContext,
    host_context: &HostContext,
    host_binding: &HostBinding,
    changes_record: &mut ChangesRecord,
) -> Result<(), InternalError> {
    let service: HostCon = host_context.create(host_binding);
    let host_state = service.get_host_state().await.catch_infra(ctx)?;

    let service: monee::nodes::sync::application::rewrite_system::RewriteSystem = ctx.provide();
    let result = service.run(host_state).await.catch_infra(ctx)?;
    if result.is_err() {
        eprintln!("WARNING: Failed to overwrite system");
    }

    *changes_record = ChangesRecord::default();

    Ok(())
}
