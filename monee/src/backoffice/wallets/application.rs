pub mod name_resolve {
    use cream::context::FromContext;

    use crate::{
        backoffice::wallets::domain::{repository::Repository, wallet_name::WalletName},
        prelude::{AppContext, InfrastructureError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct NameResolve {
        repository: Box<dyn Repository>,
    }

    impl NameResolve {
        pub async fn run(
            &self,
            name: &WalletName,
        ) -> Result<Option<monee_core::WalletId>, InfrastructureError> {
            self.repository.find_by_name(name).await
        }
    }
}

pub mod create_one {
    use cream::{context::FromContext, events::bus::EventBusPort};
    use monee_core::WalletId;

    use crate::{
        backoffice::wallets::domain::{
            repository::Repository, wallet::Wallet, wallet_created::WalletCreated,
        },
        shared::{
            domain::{context::AppContext, errors::UniqueSaveError},
            infrastructure::errors::{AppError, InfrastructureError},
        },
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct CreateOne {
        repository: Box<dyn Repository>,
        bus: EventBusPort,
    }

    impl CreateOne {
        pub async fn run(&self, wallet: Wallet) -> Result<(), AppError<UniqueSaveError>> {
            let id = WalletId::new();
            let currency_id = wallet.currency_id;

            self.repository.save(id, wallet).await?;
            self.bus.publish(WalletCreated { id, currency_id });

            Ok(())
        }
    }

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error(transparent)]
        Unspecified(#[from] InfrastructureError),
        #[error("Wallet name already exists")]
        AlreadyExists,
    }
}

pub mod update_one {
    use cream::context::FromContext;
    use monee_core::WalletId;

    use crate::{
        backoffice::wallets::domain::{
            repository::{Repository, UpdateError},
            wallet_name::WalletName,
        },
        shared::domain::context::AppContext,
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct UpdateOne {
        repository: Box<dyn Repository>,
    }

    impl UpdateOne {
        pub async fn run(
            &self,
            id: WalletId,
            name: Option<WalletName>,
            description: String,
        ) -> Result<(), UpdateError> {
            self.repository.update(id, name, description).await?;
            Ok(())
        }
    }
}
