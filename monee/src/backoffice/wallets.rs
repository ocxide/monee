pub mod application {
    pub mod create_one {
        use cream::from_context::FromContext;

        use crate::{
            backoffice::wallets::domain::{repository::Repository, wallet::Wallet},
            shared::{domain::context::AppContext, errors::InfrastructureError},
        };

        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl<C: AppContext> FromContext<C> for CreateOne {
            fn from_context(context: &C) -> Self {
                Self {
                    repository: context.backoffice_wallets_repository(),
                }
            }
        }

        impl CreateOne {
            pub async fn run(&self, wallet: Wallet) -> Result<(), InfrastructureError> {
                self.repository.save(wallet).await?;

                Ok(())
            }
        }
    }
}

pub mod domain {
    pub mod repository {
        use monee_core::WalletId;

        use super::wallet::Wallet;

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                wallet: Wallet,
            ) -> Result<WalletId, crate::shared::errors::InfrastructureError>;
        }
    }

    pub mod wallet {
        pub struct Wallet {
            pub currency_id: monee_core::CurrencyId,
            pub name: Option<String>,
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use crate::shared::infrastructure::database::Connection;

        pub struct SurrealRepository(Connection);
    }
}
