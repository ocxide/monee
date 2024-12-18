use monee::nodes::hosts::domain::host::host_binding::HostBinding;
use tokio::sync::{mpsc, watch};

use crate::prelude::InternalError;

pub struct HostSync {
    info_tx: mpsc::Sender<Option<HostBinding>>,
    pub sycn_confirm_rx: SyncConfirmer,
}

impl HostSync {
    pub fn create() -> (
        Self,
        mpsc::Receiver<Option<HostBinding>>,
        watch::Sender<Result<(), InternalError>>,
    ) {
        let (info_tx, info_rx) = mpsc::channel(1);
        let (sycn_confirm_tx, sycn_confirm_rx) = watch::channel(Ok(()));

        let me = Self {
            info_tx,
            sycn_confirm_rx: SyncConfirmer(sycn_confirm_rx),
        };

        (me, info_rx, sycn_confirm_tx)
    }

    pub async fn set_binding(&mut self, binding: HostBinding) -> Result<(), InternalError> {
        self.info_tx
            .send(Some(binding))
            .await
            .expect("Failed to send");

        self.sycn_confirm_rx.wait_sync().await
    }
}

#[derive(Clone)]
pub struct SyncConfirmer(watch::Receiver<Result<(), InternalError>>);

impl SyncConfirmer {
    pub async fn wait_sync(&mut self) -> Result<(), InternalError> {
        self.0.changed().await.expect("Failed to read to changes");
        self.0.borrow().as_ref().copied().map_err(|e| e.clone())
    }
}
