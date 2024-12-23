pub use monee_types::reports::snapshot::*;
pub mod repository {
    use monee_core::WalletId;
    use monee_types::reports::snapshot::snapshot::{Money, Wallet};

    use crate::shared::infrastructure::errors::InfrastructureError;

    use super::snapshot::Snapshot;

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn read(&self) -> Result<Snapshot, InfrastructureError>;
        async fn get_wallets(
            &self,
        ) -> Result<Vec<(WalletId, (Wallet, Money))>, InfrastructureError>;
    }
}

