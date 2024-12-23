use monee::{
    nodes::hosts::{
        application::save_host_dir::SetHostBinding, domain::host::host_binding::HostBinding,
    },
    prelude::*,
};
use tokio::sync::Mutex;

use crate::{
    host_interop::host_sync::{HostSync, SyncConfirmer},
    prelude::*,
};

pub struct HostSyncState {
    host_sync: Mutex<HostSync>,
    ctx: AppContext,
}

impl HostSyncState {
    pub fn new(ctx: AppContext, host_sync: HostSync) -> Self {
        Self {
            host_sync: Mutex::new(host_sync),
            ctx,
        }
    }

    pub async fn set_binding(&self, binding: HostBinding) -> Result<(), InternalError> {
        let service: SetHostBinding = self.ctx.provide();
        service.run(&binding).await.catch_infra(&self.ctx)?;

        self.send_binding(binding).await
    }

    pub async fn send_binding(&self, binding: HostBinding) -> Result<(), InternalError> {
        self.host_sync.lock().await.set_binding(binding).await
    }
}

pub struct SyncConfirmState(Mutex<SyncConfirmer>);

impl SyncConfirmState {
    pub fn new(sync_confirmer: SyncConfirmer) -> Self {
        Self(Mutex::new(sync_confirmer))
    }

    pub async fn wait_sync(&self) -> Result<(), InternalError> {
        self.0.lock().await.wait_sync().await
    }
}

pub fn setup(ctx: AppContext, host_sync: HostSync) -> (SyncConfirmState, HostSyncState) {
    let sync_confirmer = SyncConfirmState::new(host_sync.sycn_confirm_rx.clone());
    let host_sync = HostSyncState::new(ctx.clone(), host_sync);

    (sync_confirmer, host_sync)
}

