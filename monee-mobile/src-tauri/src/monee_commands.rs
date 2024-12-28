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

use monee::nodes::hosts::domain::host::host_dir::HostDir;
use monee::reports::wallets::application::get_all as get_all_wallets;
use monee::shared::domain::errors::UniqueSaveError;
use monee_core::{ActorId, ItemTagId, WalletId};

use monee::backoffice::actors::application::get_all as get_all_actors;

use crate::host_interop::host_context::{HostContext, RegisterNode};
use crate::host_sync_state::HostSyncState;
use crate::prelude::*;

use monee::backoffice::actors::application::create_one as create_actor;
use monee::backoffice::item_tags::application::create_one as create_item;

write_command!(add_event::Add : add_event( event: add_event::Event ) -> (), MoneeError<add_event::Error>);
read_command!(get_all_items::GetAll : get_all_items() -> Vec<ItemTagNode>, InternalError);
read_command!(get_all_wallets::GetAll : get_all_wallets() -> Vec<(WalletId, (get_all_wallets::Wallet, get_all_wallets::Money))>, InternalError);
read_command!(get_all_actors::GetAll : get_all_actors() -> Vec<(ActorId, Actor)>, InternalError);
write_command!(create_item::CreateOne : create_item(item: ItemTag) -> ItemTagId, MoneeError<UniqueSaveError>);
write_command!(create_actor::CreateOne : create_actor(actor: Actor) -> ActorId, MoneeError<UniqueSaveError>);

#[tauri::command]
pub async fn get_stats(
    ctx: tauri::State<'_, AppContext>,
) -> Result<monee::reports::snapshot::domain::snapshot::Snapshot, InternalError> {
    let service: monee::reports::snapshot::application::snapshot_report::SnapshotReport =
        ctx.provide();

    service.run().await.catch_infra(&ctx)
}

#[tauri::command]
pub async fn set_host(
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
pub async fn is_synced(
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
