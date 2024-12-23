pub mod events;
pub mod snapshot;
pub mod wallets {
    pub mod application {
        pub mod get_all {
            use monee_core::WalletId;
            pub use monee_types::reports::snapshot::snapshot::{Money, Wallet};

            use crate::reports::snapshot::domain::repository::Repository;
            pub use crate::{iprelude::*, prelude::*};

            #[derive(FromContext)]
            #[context(AppContext)]
            pub struct GetAll {
                repository: Box<dyn Repository>,
            }

            impl GetAll {
                pub async fn run(
                    &self,
                ) -> Result<Vec<(WalletId, (Wallet, Money))>, InfrastructureError> {
                    self.repository.get_wallets().await
                }
            }
        }
    }
}
