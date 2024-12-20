use cream::context::{Context, CreateFromContext, FromContext};
use monee::{
    host::{
        nodes::domain::app_id::AppId,
        sync::domain::{sync_guide::SyncGuide, sync_report::SyncReport, sync_save::SyncSave},
    },
    nodes::hosts::domain::host::{host_binding::HostBinding, host_dir::HostDir},
    prelude::InfrastructureError,
};
use tauri_plugin_http::reqwest::Client;

use crate::prelude::*;

#[derive(Clone, Default)]
pub struct HostContext {
    http: Client,
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
    pub async fn get_guide(&self) -> Result<SyncGuide, InfrastructureError> {
        let sync_guide = self
            .http
            .get(format!("{}/sync/guide", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
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
            .get(format!("{}/sync/report", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
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
            .post(format!("{}/sync/save", self.info.dir))
            .header("X-Node-Id", self.info.node_app_id.to_string())
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await
            .catch_to_infra()?;

        Ok(())
    }
}

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