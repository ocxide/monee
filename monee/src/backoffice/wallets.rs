pub mod application {
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
        use cream::{context::FromContext, event_bus::EventBusPort};
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
}

pub mod domain {
    pub mod repository {
        use monee_core::WalletId;

        use crate::shared::{
            domain::errors::UniqueSaveError,
            infrastructure::errors::{AppError, InfrastructureError},
        };

        use super::{wallet::Wallet, wallet_name::WalletName};

        #[async_trait::async_trait]
        pub trait Repository {
            async fn save(
                &self,
                id: WalletId,
                wallet: Wallet,
            ) -> Result<(), AppError<UniqueSaveError>>;

            async fn update(
                &self,
                id: WalletId,
                name: Option<WalletName>,
                description: String,
            ) -> Result<(), UpdateError>;

            async fn find_by_name(
                &self,
                name: &WalletName,
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

    pub mod wallet {
        use super::wallet_name::WalletName;

        pub struct Wallet {
            pub currency_id: monee_core::CurrencyId,
            pub name: WalletName,
            pub description: String,
        }
    }

    pub mod wallet_name {
        use std::{fmt::Display, str::FromStr};

        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        pub struct WalletName(String);

        impl Display for WalletName {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error("Invalid character: {0:?}")]
            InvalidCharacter(char),
        }

        impl TryFrom<String> for WalletName {
            type Error = Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                match value
                    .chars()
                    .find(|c| !matches!(*c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'))
                {
                    Some(c) => Err(Error::InvalidCharacter(c)),
                    None => Ok(Self(value)),
                }
            }
        }

        impl FromStr for WalletName {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s.to_string())
            }
        }
    }

    pub mod wallet_created {
        use cream::events::DomainEvent;

        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        pub struct WalletCreated {
            pub id: monee_core::WalletId,
            pub currency_id: monee_core::CurrencyId,
        }

        impl DomainEvent for WalletCreated {
            fn name(&self) -> &'static str {
                "backoffice.wallets.created"
            }

            fn version(&self) -> &'static str {
                "1.0.0"
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::FromContext;
        use monee_core::WalletId;

        use crate::{
            backoffice::wallets::domain::{
                repository::{Repository, UpdateError},
                wallet::Wallet,
                wallet_name::WalletName,
            },
            shared::{
                domain::{context::DbContext, errors::UniqueSaveError},
                infrastructure::{
                    database::{Connection, EntityKey},
                    errors::{AppError, InfrastructureError, IntoAppResult},
                },
            },
        };

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl Repository for SurrealRepository {
            async fn save(
                &self,
                id: WalletId,
                wallet: Wallet,
            ) -> Result<(), AppError<UniqueSaveError>> {
                let result = self.0
                    .query("CREATE ONLY type::thing('wallet', $id) SET currency_id = type::thing('currency', $currency_id), name = $name, description = $description")
                    .bind(("id", id))
                    .bind(("currency_id", wallet.currency_id))
                    .bind(("name", wallet.name))
                    .bind(("description", wallet.description))
                    .await.map_err(InfrastructureError::from)?
                    .check();

                result.into_app_result()
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
                    .await.map_err(|e| UpdateError::Unspecified(e.into()))?.check();

                match result {
                    Ok(mut response) => match response
                        .take(0)
                        .map_err(|e| UpdateError::Unspecified(e.into()))?
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
                    Err(e) => Err(UpdateError::Unspecified(e.into())),
                }
            }

            async fn find_by_name(
                &self,
                name: &WalletName,
            ) -> Result<Option<WalletId>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT VALUE id FROM wallet WHERE name = $name")
                    .bind(("name", name))
                    .await
                    .map_err(InfrastructureError::from)?;

                let wallet_id: Option<EntityKey<WalletId>> = response.take(0).map_err(InfrastructureError::from)?;
                Ok(wallet_id.map(|w| w.0))
            }
        }

        #[cfg(all(test, feature = "db_test"))]
        mod test {
            use monee_core::CurrencyId;

            use super::*;

            #[test]
            fn can_save() {
                return;
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    let db = crate::shared::infrastructure::database::connect().await.unwrap();
                    let ctx = crate::shared::domain::context::DbContext::new(db.clone());
                    let wallet_repo: crate::backoffice::wallets::infrastructure::repository::SurrealRepository = ctx.provide();

                    let id = WalletId::new();
                    let wallet = Wallet {
                        currency_id: CurrencyId::new(),
                        name: "foo".parse().unwrap(),
                        description: "description".into(),
                    };
                    wallet_repo.save(id, wallet).await.unwrap();

                    let mut response = db.query("SELECT count() as count FROM wallet").await.unwrap();
                    let count: Option<i32> = response.take("count").unwrap();

                    assert_eq!(count, Some(1));
                });
            }
        }
    }
}
