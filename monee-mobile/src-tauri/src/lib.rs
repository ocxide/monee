use cream::events::multi_dispatch_listener::{MultiDispatchListener, MultiDispatchers};
use host_interop::{host_context::HostContext, node_sync::NodeSyncContext};
use monee::shared::domain::context::AppContextBuilder;
use tauri::Manager;

use prelude::*;

mod host_interop;
mod host_sync_state;
mod monee_commands;
mod prelude;

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
    dbg!(&base_dir);
    let setup = AppContextBuilder { base_dir }.build().await?;

    let host_ctx = HostContext::default();

    let (data_port, host_sync) =
        host_interop::node_sync::setup(setup.ctx.clone(), host_ctx.clone(), app.handle().clone());
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| tauri::async_runtime::block_on(setup(app)))
        .invoke_handler(tauri::generate_handler![
            monee_commands::get_stats,
            monee_commands::set_host,
            monee_commands::is_synced,
            monee_commands::add_event,
            monee_commands::get_all_items,
            monee_commands::get_all_wallets,
            monee_commands::get_all_actors,
            monee_commands::create_item,
            monee_commands::create_actor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
