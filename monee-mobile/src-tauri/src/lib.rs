use cream::events::multi_dispatch_listener::{MultiDispatchListener, MultiDispatchers};
use host_interop::{
    host_context::{HostContext, RegisterNode},
    node_sync::NodeSyncContext,
};
use host_sync_state::HostSyncState;
use monee::{
    nodes::hosts::domain::host::host_dir::HostDir, shared::domain::context::AppContextBuilder,
};
use tauri::{Emitter, Manager};

use prelude::*;

mod host_interop;
mod host_sync_state;
mod prelude;
mod monee_commands {
    macro_rules! write_command {
        ($service: path: $name: ident( $( $arg: ident: $arg_type: path ),*) -> $ret: ty, $err: ty ) => {
            #[tauri::command]
            pub async fn $name(
                $( $arg: $arg_type, )*
                ctx: tauri::State<'_, monee::prelude::AppContext>,
                confirmer: tauri::State<'_, crate::host_sync_state::SyncConfirmState>,
            ) -> Result<$ret, $err> {
                use monee::prelude::*;
                let service: $service = ctx.provide();
                #[allow(clippy::let_unit_value)]
                let response = service.run( $( $arg ),* ).await?;

                confirmer.wait_sync().await?;

                Ok(response)
            }
        };
    }

    macro_rules! read_command {
        ($service: path: $name: ident( $( $arg: ident: $arg_type: path ),*) -> $ret: ty, $err: ty ) => {
            #[tauri::command]
            pub async fn $name(
               $( $arg: $arg_type, )*
               ctx: tauri::State<'_, monee::prelude::AppContext>,
            ) -> Result<$ret, $err> {
               use monee::prelude::*;
               let service: $service = ctx.provide();
               service.run( $( $arg ),* ).await.catch_infra(&ctx)
            }
        };
    }

    use monee::backoffice::item_tags::domain::item_tag::ItemTag;
    use monee::backoffice::{actors::domain::actor::Actor, events::application::add as add_event};

    use monee::backoffice::item_tags::application::get_all as get_all_items;
    use monee::backoffice::item_tags::domain::item_tag_node::ItemTagNode;

    use monee::reports::wallets::application::get_all as get_all_wallets;
    use monee::shared::domain::errors::UniqueSaveError;
    use monee_core::{ActorId, ItemTagId, WalletId};

    use monee::backoffice::actors::application::get_all as get_all_actors;

    use crate::prelude::*;

    use monee::backoffice::actors::application::create_one as create_actor;
    use monee::backoffice::item_tags::application::create_one as create_item;

    write_command!(add_event::Add : add_event( event: add_event::Event ) -> (), MoneeError<add_event::Error>);
    read_command!(get_all_items::GetAll : get_all_items() -> Vec<ItemTagNode>, InternalError);
    read_command!(get_all_wallets::GetAll : get_all_wallets() -> Vec<(WalletId, (get_all_wallets::Wallet, get_all_wallets::Money))>, InternalError);
    read_command!(get_all_actors::GetAll : get_all_actors() -> Vec<(ActorId, Actor)>, InternalError);
    write_command!(create_item::CreateOne : create_item(item: ItemTag) -> ItemTagId, MoneeError<UniqueSaveError>);
    write_command!(create_actor::CreateOne : create_actor(actor: Actor) -> ActorId, MoneeError<UniqueSaveError>);
}

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
        .invoke_handler(tauri::generate_handler![
            get_stats,
            set_host,
            is_synced,
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
