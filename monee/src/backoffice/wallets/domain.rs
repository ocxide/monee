pub use monee_types::backoffice::wallets::*;
pub mod repository {
    use monee_core::WalletId;

    use crate::shared::{
        domain::errors::UniqueSaveError,
        infrastructure::errors::{AppError, InfrastructureError},
    };

    use super::{wallet::Wallet, wallet_name::WalletName};

    #[async_trait::async_trait]
    pub trait Repository: 'static + Send + Sync {
        async fn save(&self, id: WalletId, wallet: Wallet)
            -> Result<(), AppError<UniqueSaveError>>;

        async fn update(
            &self,
            id: WalletId,
            name: Option<WalletName>,
            description: String,
        ) -> Result<(), UpdateError>;

        async fn find_by_name(
            &self,
            name: WalletName,
        ) -> Result<Option<WalletId>, InfrastructureError>;
    }

    #[derive(thiserror::Error, Debug)]
    pub enum UpdateError {
        #[error("Wallet id not found")]
        NotFound,
        #[error("Wallet name already exists")]
        AlreadyExists,
        #[error(transparent)]
        Unspecified(InfrastructureError),
    }
}

