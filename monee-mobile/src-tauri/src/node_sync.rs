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

mod host_context {
    use cream::context::{Context, CreateFromContext, FromContext};
    use monee::{
        host::{
            nodes::domain::app_id::AppId,
            sync::domain::{sync_guide::SyncGuide, sync_report::SyncReport, sync_save::SyncSave},
        },
        nodes::hosts::domain::host::host_dir::HostDir,
        prelude::InfrastructureError,
    };
    use tauri_plugin_http::reqwest::Client;

    use super::{CatchInfra, CatchToInfra};

    #[derive(Clone, Default)]
    pub struct HostContext {
        http: Client,
    }

    impl Context for HostContext {}

    pub struct HostInfo {
        host_dir: HostDir,
        app_id: AppId,
    }
    pub struct HostCon<'a> {
        info: &'a HostInfo,
        http: Client,
    }

    impl<'a> CreateFromContext<HostContext> for HostCon<'a> {
        type Args = &'a HostInfo;
        fn create_from_context(ctx: &HostContext, args: Self::Args) -> Self {
            Self {
                info: args,
                http: ctx.http.clone(),
            }
        }
    }

    impl<'a> HostCon<'a> {
        pub async fn get_guide(&self) -> Result<SyncGuide, InfrastructureError> {
            let sync_guide = self
                .http
                .get(format!("{}/sync/guide", self.info.host_dir))
                .header("X-Node-Id", self.info.app_id.to_string())
                .send()
                .await
                .catch_to_infra()?
                .json::<SyncGuide>()
                .await
                .catch_to_infra()?;

            Ok(sync_guide)
        }

        pub async fn get_report(&self) -> Result<SyncReport, InfrastructureError> {
            let sync_report = self
                .http
                .get(format!("{}/sync/report", self.info.host_dir))
                .header("X-Node-Id", self.info.app_id.to_string())
                .send()
                .await
                .catch_to_infra()?
                .json::<SyncReport>()
                .await
                .catch_to_infra()?;

            Ok(sync_report)
        }

        pub async fn sync_to_host(&self, data: &SyncSave) -> Result<(), InfrastructureError> {
            self.http
                .post(format!("{}/sync/save", self.info.host_dir))
                .header("X-Node-Id", self.info.app_id.to_string())
                .header("Content-Type", "application/json")
                .json(&data)
                .send()
                .await
                .catch_to_infra()?;

            Ok(())
        }

        pub async fn register_self(&self) -> Result<AppId, InfrastructureError> {
            let app_id = self
                .http
                .post(format!("{}/nodes", self.info.host_dir))
                .header("Content-Type", "application/json")
                .send()
                .await
                .catch_to_infra()?
                .json::<AppId>()
                .await
                .catch_to_infra()?;

            Ok(app_id)
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

#[derive(Clone)]
pub struct HostConPort(Sender<Option<(AppId, HostDir)>>);

impl HostConPort {
    pub async fn send(&self, value: Option<(AppId, HostDir)>) {
        self.0.send(value).await.expect("Failed to send");
    }
}
type HostConRx = Receiver<Option<(AppId, HostDir)>>;

pub struct Synced;
pub enum SyncedEvent {}

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
    let Ok(report) = http_client
        .get(format!("{host_dir}/sync/report"))
        .header("X-Node-Id", app_id.to_string())
        .send()
        .await
        .unwrap()
        .json::<SyncReport>()
        .await
    else {
        return;
    };
    dbg!(&report);

    let service: monee::nodes::sync::application::rewrite_system::RewriteSystem = ctx.provide();
    let result = service.run(report).await.catch_infra(ctx).unwrap();
    if let Err(_) = result {
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
