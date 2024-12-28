use std::time::Duration;

use cream::context::{Context, CreateFromContext, FromContext};
use monee::{
    host::{
        nodes::domain::app_id::AppId,
        sync::domain::{host_state::HostState, node_changes::NodeChanges, sync_guide::SyncGuide},
    },
    nodes::hosts::domain::host::{host_binding::HostBinding, host_dir::HostDir},
    prelude::InfrastructureError,
};
use tauri_plugin_http::reqwest::{Client, ClientBuilder};

use crate::prelude::*;

#[derive(Clone)]
pub struct HostContext {
    http: Client,
}

impl Default for HostContext {
    fn default() -> Self {
        let http = ClientBuilder::default()
            .connect_timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to build http client");
        Self { http }
    }
}

impl Context for HostContext {}

impl FromContext<HostContext> for Client {
    fn from_context(ctx: &HostContext) -> Self {
        ctx.http.clone()
    }
}

pub struct HostCon<'a> {
    info: &'a HostBinding,
    http: Client,
}

impl<'a> CreateFromContext<HostContext> for HostCon<'a> {
    type Args = &'a HostBinding;
    fn create_from_context(ctx: &HostContext, args: Self::Args) -> Self {
        Self {
            info: args,
            http: ctx.provide(),
        }
    }
}

impl<'a> HostCon<'a> {
    pub async fn get_guide(&self) -> Result<SyncGuide, AppError<ConnectError>> {
        let sync_guide = self
            .http
            .get(format!("{}/sync/guide", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
            .send()
            .await
            .catch_to_app()?
            .error_for_status()
            .catch_to_infra()?
            .json::<SyncGuide>()
            .await
            .catch_to_infra()?;

        Ok(sync_guide)
    }

    pub async fn get_host_state(&self) -> Result<HostState, AppError<ConnectError>> {
        let sync_report = self
            .http
            .get(format!("{}/sync/report", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
            .send()
            .await
            .catch_to_app()?
            .error_for_status()
            .catch_to_infra()?
            .json::<HostState>()
            .await
            .catch_to_infra()?;

        Ok(sync_report)
    }

    pub async fn sync_to_host(&self, data: &NodeChanges) -> Result<(), AppError<ConnectError>> {
        println!("CALLING API");
        self.http
            .patch(format!("{}/sync", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await
            .catch_to_app()?
            .error_for_status()
            .catch_to_infra()?;

        Ok(())
    }
}

pub struct ConnectError;

#[derive(FromContext)]
#[context(HostContext)]
pub struct RegisterNode {
    http: Client,
}

impl RegisterNode {
    pub async fn run(&self, dir: &HostDir) -> Result<AppId, InfrastructureError> {
        let app_id = self
            .http
            .post(format!("{}/nodes", dir))
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
