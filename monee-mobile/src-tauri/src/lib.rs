use monee::shared::domain::context::AppContextBuilder;
use tauri::Manager;

mod prelude;

use prelude::*;

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
    let context = AppContextBuilder { base_dir }.setup().await?;

    app.manage(context);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| tauri::async_runtime::block_on(setup(app)))
        .invoke_handler(tauri::generate_handler![get_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
