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

mod node_sync {
    use monee::{
        host::{
            nodes::domain::app_id::AppId,
            sync::domain::{sync_guide::SyncGuide, sync_report::SyncReport},
        },
        nodes::hosts::domain::{host::host_dir::HostDir, sync::changes_record::ChangesRecord},
        prelude::AppContext,
    };
    use monee_core::{ActorId, CurrencyId, WalletId};
    use tauri::async_runtime::{Receiver, Sender};
    use tauri_plugin_http::reqwest::Client;

    use crate::prelude::*;

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

    #[derive(Clone)]
    pub struct HostConPort(Sender<Option<(AppId, HostDir)>>);

    impl HostConPort {
        pub async fn send(&self, value: Option<(AppId, HostDir)>) {
            self.0.send(value).await.expect("Failed to send");
        }
    }
    type HostConRx = Receiver<Option<(AppId, HostDir)>>;

    pub fn setup(ctx: AppContext, client: Client) -> (DataChangedPort, HostConPort) {
        let (changes_tx, changes_rx) = tauri::async_runtime::channel(1);
        let (host_tx, host_rx) = tauri::async_runtime::channel(1);

        tauri::async_runtime::spawn(listen(changes_rx, host_rx, ctx, client));
        (DataChangedPort(changes_tx), HostConPort(host_tx))
    }

    async fn listen(
        mut data_rx: Receiver<DataChanged>,
        mut host_rx: HostConRx,
        ctx: AppContext,
        http_client: Client,
    ) {
        let mut changes = ChangesRecord::default();
        let mut host_con = None;

        loop {
            let should_save_changes = tokio::select! {
                data_changed = data_rx.recv() => {
                    if let Some(data_changed) = data_changed {
                        on_data_changed(&mut changes, data_changed);
                        true
                    }
                    else { false }
                },

                host_changed = host_rx.recv() => {
                    if let Some(host_changed) = host_changed {
                        host_con = host_changed;
                    }

                    false
                }
            };

            let Some((app_id, host_dir)) = host_con.as_ref() else {
                println!("WARNING: changes not sent due no host connection");
                continue;
            };

            if should_save_changes {
                save_changes(&http_client, *app_id, host_dir, &changes, &ctx)
                    .await
                    .unwrap();
            }

            get_data(&http_client, *app_id, host_dir, &ctx, &mut changes).await;
        }
    }

    async fn get_data(
        http_client: &Client,
        app_id: AppId,
        host_dir: &HostDir,
        ctx: &AppContext,
        changes: &mut ChangesRecord,
    ) {
        let report = http_client
            .get(format!("{host_dir}/sync/report"))
            .header("X-Node-Id", app_id.to_string())
            .send()
            .await
            .unwrap()
            .json::<SyncReport>()
            .await
            .unwrap();

        let service: monee::nodes::sync::application::rewrite_system::RewriteSystem = ctx.provide();
        let result = service.run(report).await.catch_infra(&ctx).unwrap();
        if let Err(e) = result {
            eprintln!("Failed to get data");
        }

        *changes = ChangesRecord::default();
    }

    async fn save_changes(
        http_client: &Client,
        app_id: AppId,
        host_dir: &HostDir,
        changes: &ChangesRecord,
        ctx: &AppContext,
    ) -> Result<(), InternalError> {
        let sync_guide = http_client
            .get(format!("{host_dir}/sync/guide"))
            .header("X-Node-Id", app_id.to_string())
            .send()
            .await
            .unwrap()
            .json::<SyncGuide>()
            .await
            .unwrap();

        let service: monee::nodes::sync::application::get_sync_save::GetSyncSave = ctx.provide();
        let sync_save = service.run(sync_guide, changes).await.catch_infra(ctx)?;

        http_client
            .post("{host_dir}/sync/save")
            .header("X-Node-Id", app_id.to_string())
            .json(&sync_save)
            .send()
            .await
            .unwrap();

        Ok(())
    }

    fn on_data_changed(changes: &mut ChangesRecord, event: DataChanged) {
        match event {
            DataChanged::Currency(id) => changes.currencies.push(id),
            DataChanged::Actor(id) => changes.actors.push(id),
            DataChanged::Wallet(id) => changes.wallets.push(id),
            // Should save too?
            DataChanged::Event => {}
        }
    }
}

struct HostState(pub Mutex<HostConnection>);
struct HostConnection(pub Option<(AppId, HostDir)>);

async fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app.path().app_data_dir().expect("AppData not found");
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
    http: tauri::State<'_, Client>,
    host_state: tauri::State<'_, HostState>,
    host_port: tauri::State<'_, HostConPort>,
    host_dir: HostDir,
) -> Result<(), InternalError> {
    let app_id = http
        .post(format!("{host_dir}/nodes"))
        .send()
        .await
        .unwrap()
        .json::<AppId>()
        .await
        .unwrap();

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
