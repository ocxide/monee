pub mod application {
    pub mod create_one {
        use cream::context::FromContext;
        use monee_core::WalletId;

        use crate::{
            backoffice::wallets::domain::{repository::Repository, wallet::Wallet},
            shared::{
                domain::context::AppContext,
                infrastructure::errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        #[derive(FromContext)]
        #[from_context(C: AppContext)]
        pub struct CreateOne {
            repository: Box<dyn Repository>,
        }

        impl CreateOne {
            pub async fn run(&self, wallet: Wallet) -> Result<(), Error> {
                self.repository
                    .save(WalletId::new(), wallet)
                    .await
                    .map_err(|e| match e {
                        UniqueSaveError::Unspecified(e) => Error::Unspecified(e),
                        UniqueSaveError::AlreadyExists => Error::AlreadyExists,
                    })?;

                Ok(())
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            Unspecified(#[from] UnspecifiedError),
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
        #[from_context(C: AppContext)]
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
}

pub mod domain {
    pub mod repository {
        use cream::context::FromContext;
        use monee_core::WalletId;

        use crate::shared::{domain::context::AppContext, errors::InfrastructureError, infrastructure::errors::UniqueSaveError};

        use super::{wallet::Wallet, wallet_name::WalletName};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(&self, id: WalletId, wallet: Wallet) -> Result<(), UniqueSaveError>;
            async fn update(
                &self,
                id: WalletId,
                name: Option<WalletName>,
                description: String,
            ) -> Result<(), UpdateError>;
        }

        impl<C: AppContext> FromContext<C> for Box<dyn Repository> {
            fn from_context(context: &C) -> Self {
                context.backoffice_wallets_repository()
            }
        }

        #[derive(thiserror::Error, Debug)]
        pub enum UpdateError {
            #[error("Wallet id not found")]
            NotFound,
            #[error("Wallet name already exists")]
            AlreadyExists,
            #[error(transparent)]
            Infrastructure(InfrastructureError),
        }
    }

    pub mod wallet {
        use super::wallet_name::WalletName;

        pub struct Wallet {
            pub currency_id: monee_core::CurrencyId,
            pub name: Option<WalletName>,
            pub description: String,
        }
    }

    pub mod wallet_name {
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct WalletName(String);

        pub enum Error {
            InvalidCharacter(char),
        }

        impl TryFrom<String> for WalletName {
            type Error = Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                match value.chars().find(|c| !c.is_alphanumeric()) {
                    Some(c) => Err(Error::InvalidCharacter(c)),
                    None => Ok(Self(value)),
                }
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use monee_core::WalletId;

        use crate::{
            backoffice::wallets::domain::{
                repository::{Repository, UpdateError},
                wallet::Wallet,
                wallet_name::WalletName,
            },
            shared::infrastructure::{
                database::Connection,
                errors::{UniqueSaveError, UnspecifiedError},
            },
        };

        pub struct SurrealRepository(Connection);
        impl SurrealRepository {
            pub(crate) fn new(
                clone: surrealdb::Surreal<surrealdb::engine::remote::ws::Client>,
            ) -> Self {
                Self(clone)
            }
        }

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(&self, id: WalletId, wallet: Wallet) -> Result<(), UniqueSaveError> {
                let result = self.0
                    .query("INSERT INTO wallet (id, currency_id, name) VALUES ($id, $currency_id, $name)")
                    .bind(("id", id))
                    .bind(("currency_id", wallet.currency_id))
                    .bind(("name", wallet.name))
                    .await.map_err(UnspecifiedError::new)?.check();

                match result {
                    Ok(_) => Ok(()),
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(UniqueSaveError::AlreadyExists),
                    Err(e) => Err(UniqueSaveError::Unspecified(e.into())),
                }
            }

            async fn update(
                &self,
                id: WalletId,
                name: Option<WalletName>,
                description: String,
            ) -> Result<(), UpdateError> {
                let result = self.0
                    .query("UPDATE type::thing('wallet', $id) SET name = $name, description = $description")
                    .bind(("id", id))
                    .bind(("name", name))
                    .bind(("description", description))
                    .await.map_err(|e| UpdateError::Infrastructure(e.into()))?.check();

                match result {
                    Ok(mut response) => match response
                        .take(0)
                        .map_err(|e| UpdateError::Infrastructure(e.into()))?
                    {
                        Some(()) => Ok(()),
                        None => Err(UpdateError::NotFound),
                    },
                    Err(
                        crate::shared::infrastructure::database::Error::Api(
                            surrealdb::error::Api::Query { .. },
                        )
                        | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                    ) => Err(UpdateError::AlreadyExists),
                    Err(e) => Err(UpdateError::Infrastructure(e.into())),
                }
            }
        }
    }
}
