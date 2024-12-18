use host_interop::{
    host_context::{HostContext, RegisterNode},
    host_sync::HostSync,
};
use monee::{
    nodes::hosts::domain::host::host_dir::HostDir, shared::domain::context::AppContextBuilder,
};
use tauri::Manager;

mod prelude;

use prelude::*;
use tokio::sync::Mutex;

mod host_interop;

pub struct HostSyncState(Mutex<HostSync>);

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
    dbg!(&base_dir);
    let context = AppContextBuilder { base_dir }.setup().await?;

    let host_ctx = HostContext::default();

    let (data_port, host_sync) = host_interop::node_sync::setup(context.clone(), host_ctx.clone());

    app.manage(data_port);
    app.manage(HostSyncState(Mutex::new(host_sync)));
    app.manage(context);
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
        .0
        .lock()
        .await
        .set_binding(
            monee::nodes::hosts::domain::host::host_binding::HostBinding {
                dir: host_dir,
                node_app_id,
            },
        )
        .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| tauri::async_runtime::block_on(setup(app)))
        .invoke_handler(tauri::generate_handler![get_stats, set_host])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
