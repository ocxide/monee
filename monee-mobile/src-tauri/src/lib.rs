use cream::events::multi_dispatch_listener::{MultiDispatchListener, MultiDispatchers};
use host_interop::{
    host_context::{HostContext, RegisterNode},
    node_sync::NodeSyncContext,
};
use host_sync_state::HostSyncState;
use monee::{
    nodes::hosts::domain::host::host_dir::HostDir, shared::domain::context::AppContextBuilder,
};
use tauri::Manager;

mod prelude;

use prelude::*;

mod host_interop;

mod host_sync_state {
    use monee::{
        nodes::hosts::{
            application::save_host_dir::SetHostBinding, domain::host::host_binding::HostBinding,
        },
        prelude::*,
    };
    use tokio::sync::Mutex;

    use crate::{
        host_interop::host_sync::{HostSync, SyncConfirmer},
        prelude::*,
    };

    pub struct HostSyncState {
        host_sync: Mutex<HostSync>,
        ctx: AppContext,
    }

    impl HostSyncState {
        pub fn new(ctx: AppContext, host_sync: HostSync) -> Self {
            Self {
                host_sync: Mutex::new(host_sync),
                ctx,
            }
        }

        pub async fn set_binding(&self, binding: HostBinding) -> Result<(), InternalError> {
            let service: SetHostBinding = self.ctx.provide();
            service.run(&binding).await.catch_infra(&self.ctx)?;

            self.send_binding(binding).await
        }

        pub async fn send_binding(&self, binding: HostBinding) -> Result<(), InternalError> {
            self.host_sync.lock().await.set_binding(binding).await
        }
    }

    pub struct SyncConfirmState(Mutex<SyncConfirmer>);

    impl SyncConfirmState {
        pub fn new(sync_confirmer: SyncConfirmer) -> Self {
            Self(Mutex::new(sync_confirmer))
        }

        pub async fn wait_sync(&mut self) -> Result<(), InternalError> {
            self.0.lock().await.wait_sync().await
        }
    }

    pub fn setup(ctx: AppContext, host_sync: HostSync) -> (SyncConfirmState, HostSyncState) {
        let sync_confirmer = SyncConfirmState::new(host_sync.sycn_confirm_rx.clone());
        let host_sync = HostSyncState::new(ctx.clone(), host_sync);

        (sync_confirmer, host_sync)
    }
}

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
    dbg!(&base_dir);
    let setup = AppContextBuilder { base_dir }.build().await?;

    let host_ctx = HostContext::default();

    let (data_port, host_sync) =
        host_interop::node_sync::setup(setup.ctx.clone(), host_ctx.clone());
    let (sync_confirmer, host_sync) = host_sync_state::setup(setup.ctx.clone(), host_sync);

    let ctx = setup
        .cfg_events({
            let data_port = data_port.clone();
            move |cfg| {
                let mut multi = MultiDispatchers::default();
                multi.add(cfg.ctx.clone(), cfg.dispatcher);

                let (node_sync_ctx, dispatcher) = NodeSyncContext::setup(data_port);
                multi.add(node_sync_ctx, dispatcher);

                cfg.events_setup.build::<MultiDispatchListener>(multi)
            }
        })
        .setup();

    app.manage(data_port);
    app.manage(sync_confirmer);
    app.manage(host_sync);
    app.manage(ctx);
    app.manage(host_ctx);

    Ok(())
}

#[tauri::command]
async fn get_stats(
    ctx: tauri::State<'_, AppContext>,
) -> Result<monee::reports::snapshot::domain::snapshot::Snapshot, InternalError> {
    let service: monee::reports::snapshot::application::snapshot_report::SnapshotReport =
        ctx.provide();

    service.run().await.catch_infra(&ctx)
}

#[tauri::command]
async fn set_host(
    host_dir: HostDir,
    host_sync: tauri::State<'_, HostSyncState>,
    host_ctx: tauri::State<'_, HostContext>,
    ctx: tauri::State<'_, AppContext>,
) -> Result<(), InternalError> {
    let service: RegisterNode = host_ctx.provide();
    let node_app_id = service.run(&host_dir).await.catch_infra(&ctx)?;

    host_sync
        .set_binding(
            monee::nodes::hosts::domain::host::host_binding::HostBinding {
                dir: host_dir,
                node_app_id,
            },
        )
        .await
}

#[tauri::command]
async fn is_synced(
    host_sync: tauri::State<'_, HostSyncState>,
    ctx: tauri::State<'_, AppContext>,
) -> Result<bool, InternalError> {
    let getter: monee::nodes::hosts::application::get_host_info::GetHostBinding = ctx.provide();

    let Some(host_binding) = getter.run().await.catch_infra(&ctx)? else {
        return Ok(false);
    };

    host_sync.send_binding(host_binding).await?;

    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| tauri::async_runtime::block_on(setup(app)))
        .invoke_handler(tauri::generate_handler![get_stats, set_host, is_synced])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
