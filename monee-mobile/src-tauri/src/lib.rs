use std::sync::Mutex;

use monee::{
    host::nodes::domain::app_id::AppId, nodes::hosts::domain::host::host_dir::HostDir,
    shared::domain::context::AppContextBuilder,
};
use node_sync::HostConPort;
use tauri::Manager;

mod prelude;

use prelude::*;
use tauri_plugin_http::reqwest::Client;

mod node_sync;

struct HostState(pub Mutex<HostConnection>);
struct HostConnection(pub Option<(AppId, HostDir)>);

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
    dbg!(&base_dir);
    let context = AppContextBuilder { base_dir }.setup().await?;

    let http_client = tauri_plugin_http::reqwest::Client::default();

    let (data_port, host_port) = node_sync::setup(context.clone(), http_client.clone());

    app.manage(data_port);
    app.manage(host_port);
    app.manage(context);
    app.manage(http_client);
    app.manage(HostState(Mutex::new(HostConnection(None))));

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
    http: tauri::State<'_, Client>,
    host_state: tauri::State<'_, HostState>,
    host_port: tauri::State<'_, HostConPort>,
) -> Result<(), InternalError> {
    dbg!(&host_dir);
    let app_id = http
        .post(format!("{host_dir}/nodes"))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|_| InternalError::Unknown)?
        .json::<AppId>()
        .await
        .map_err(|_| InternalError::Unknown)?;

    println!("AppId: {app_id}");

    host_state.0.lock().unwrap().0 = Some((app_id, host_dir.clone()));

    host_port.send(Some((app_id, host_dir))).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
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
