mod sql_id;

pub mod database;
pub mod error;
pub mod log;
pub mod ops;

pub mod actions;
pub mod procedures;

pub mod snapshot_io;

use std::{fs, path::PathBuf};

pub use database::connect;

pub mod date {
    pub type Datetime = chrono::DateTime<chrono::Utc>;
    pub use chrono::Utc as Timezone;
}

fn create_local_path() -> PathBuf {
    let share_dir = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local/share"))
        })
        .expect("To get share directory");
    let path = share_dir.join("monee");

    fs::create_dir_all(&path).expect("To create monee data directory");
    path
}

pub use database::Entity;

pub mod backoffice {
    pub mod wallets {
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
    }

    pub mod events {
        pub mod application {
            pub mod add_buy {
                use cream::from_context::FromContext;

                use crate::{
                    backoffice::events::domain::{event::Buy, repository::Repository},
                    shared::{domain::context::AppContext, errors::InfrastructureError},
                };

                pub struct AddBuy {
                    repository: Box<dyn Repository>,
                }

                impl<C: AppContext> FromContext<C> for AddBuy {
                    fn from_context(context: &C) -> Self {
                        Self {
                            repository: context.backoffice_events_repository(),
                        }
                    }
                }

                impl AddBuy {
                    pub async fn run(&self, event: Buy) -> Result<(), InfrastructureError> {
                        self.repository.add_buy(event).await
                    }
                }
            }
        }

        pub mod domain {
            pub mod repository {
                use crate::shared::errors::InfrastructureError;

                use super::event::Buy;

                #[async_trait::async_trait]
                pub trait Repository {
                    async fn add_buy(&self, event: Buy) -> Result<(), InfrastructureError>;
                }
            }

            pub mod event {
                use monee_core::{
                    actor::ActorId, item_tag::ItemTagId, Amount, CurrencyId, WalletId,
                };

                use crate::date::Datetime;

                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct DebtRegister {
                    pub amount: Amount,
                    pub currency: CurrencyId,
                    pub actor_id: ActorId,
                    pub payment_promise: Option<Datetime>,
                }

                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct Buy {
                    pub item: ItemTagId,
                    pub actors: Box<[ActorId]>,
                    pub wallet_id: WalletId,
                    pub amount: Amount,
                }

                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct MoveValue {
                    pub from: WalletId,
                    pub to: WalletId,
                    pub amount: Amount,
                }

                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct RegisterBalance {
                    pub wallet_id: WalletId,
                    pub amount: Amount,
                }

                #[derive(serde::Serialize, serde::Deserialize)]
                pub enum Event {
                    Buy(Buy),
                    MoveValue(MoveValue),
                    RegisterBalance(RegisterBalance),
                    RegisterDebt(DebtRegister),
                    RegisterLoan(DebtRegister),
                }
            }
        }

        pub mod infrastructure {
            pub mod repository {
                use crate::backoffice::events::domain::{event::Buy, repository::Repository};

                pub struct SurrealRepository(crate::database::Connection);

                #[async_trait::async_trait]
                impl Repository for SurrealRepository {
                    async fn add_buy(
                        &self,
                        event: Buy,
                    ) -> Result<(), crate::shared::errors::InfrastructureError>
                    {
                        todo!()
                    }
                }
            }
        }
    }
}

pub mod shared {
    pub mod errors {
        #[derive(Debug, thiserror::Error)]
        #[error("infrastructure error: {0}")]
        pub struct InfrastructureError(Box<dyn std::error::Error>);

        impl InfrastructureError {
            pub fn new<E>(error: E) -> Self
            where
                E: Into<Box<dyn std::error::Error>>,
            {
                Self(error.into())
            }
        }

        #[derive(Debug, thiserror::Error)]
        pub enum SnapshotOptError {
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),

            #[error(transparent)]
            SnapshotApply(#[from] monee_core::Error),

            #[error(transparent)]
            Write(#[from] std::io::Error),

            #[error(transparent)]
            Read(#[from] crate::snapshot_io::ReadError),
        }

        #[derive(Debug, thiserror::Error)]
        pub enum SnapshotWriteError {
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),

            #[error(transparent)]
            SnapshotApply(#[from] monee_core::Error),

            #[error(transparent)]
            Write(#[from] std::io::Error),
        }

        impl From<SnapshotWriteError> for SnapshotOptError {
            fn from(value: SnapshotWriteError) -> Self {
                match value {
                    SnapshotWriteError::Infrastructure(error) => Self::Infrastructure(error),
                    SnapshotWriteError::SnapshotApply(error) => Self::SnapshotApply(error),
                    SnapshotWriteError::Write(error) => Self::Write(error),
                }
            }
        }

        impl From<SnapshotReadError> for SnapshotOptError {
            fn from(value: SnapshotReadError) -> Self {
                match value {
                    SnapshotReadError::Infrastructure(error) => Self::Infrastructure(error),
                    SnapshotReadError::SnapshotApply(error) => Self::SnapshotApply(error),
                    SnapshotReadError::Read(error) => Self::Read(error),
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        pub enum SnapshotReadError {
            #[error(transparent)]
            Infrastructure(#[from] InfrastructureError),

            #[error(transparent)]
            SnapshotApply(#[from] monee_core::Error),

            #[error(transparent)]
            Read(#[from] crate::snapshot_io::ReadError),
        }
    }

    pub mod application {
        pub mod cannonical_context {
            use cream::{context::Context, cream_context::CreamContext};

            use crate::{
                connect,
                shared::{domain::context::AppContext, errors::InfrastructureError},
            };

            #[derive(Clone)]
            pub struct CannocalContext {
                cream_context: CreamContext,
                database: crate::database::Connection,
            }

            #[derive(Debug, thiserror::Error)]
            pub enum Error {
                #[error(transparent)]
                Infrastructure(#[from] InfrastructureError),
            }

            pub async fn setup(
            ) -> Result<(CannocalContext, impl std::future::Future<Output = ()>), Error>
            {
                let db = connect().await.map_err(InfrastructureError::new)?;

                let router = cream::event_router::EventRouter::default();
                // Add event handlers

                let (port, socket) = cream::event_bus::create_channel();

                let ctx = CannocalContext {
                    cream_context: CreamContext::new(port),
                    database: db,
                };

                let listen = {
                    let ctx = ctx.clone();
                    async move {
                        cream::event_bus::EventBus::new(socket, ctx, router)
                            .listen()
                            .await;
                    }
                };

                Ok((ctx, listen))
            }

            impl Context for CannocalContext {}

            impl AppContext for CannocalContext {
                fn backoffice_events_repository(
                    &self,
                ) -> Box<dyn crate::backoffice::events::domain::repository::Repository>
                {
                    todo!()
                }

                fn backoffice_wallets_repository(
                    &self,
                ) -> Box<dyn crate::backoffice::wallets::domain::repository::Repository>
                {
                    todo!()
                }
            }
        }
    }

    pub mod domain {
        pub mod context {
            use cream::context::Context;

            pub trait AppContext: Context {
                fn backoffice_events_repository(
                    &self,
                ) -> Box<dyn crate::backoffice::events::domain::repository::Repository>;

                fn backoffice_wallets_repository(
                    &self,
                ) -> Box<dyn crate::backoffice::wallets::domain::repository::Repository>;
            }
        }
    }

    pub mod infrastructure {}
}
