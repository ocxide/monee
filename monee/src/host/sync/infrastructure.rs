pub mod repository {
    use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};
    use monee_types::{
        apps::app_id::AppId,
        backoffice::{
            actors::actor::Actor,
            currencies::currency::Currency,
            item_tags::item_tag::ItemTag,
            wallets::{wallet::Wallet, wallet_name::WalletName},
        },
    };
    use surrealdb::sql::statements::{BeginStatement, CommitStatement};

    use crate::{
        host::sync::domain::{
            repository::Repository, sync_context_data::SyncContextData, sync_error::SyncError,
            sync_guide::SyncGuide, sync_save::SyncSave,
        },
        iprelude::*,
        prelude::*,
        shared::{
            domain::{context::DbContext, date::Datetime, errors::UniqueSaveError},
            infrastructure::database::{Connection, Entity, EntityKey},
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(crate::shared::infrastructure::database::Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn get_sync_guide(&self) -> Result<SyncGuide, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT created_at FROM events ORDER BY created_at DESC LIMIT 1")
                .await?;
            let last_event_date: Option<Datetime> = response.take(0)?;

            Ok(SyncGuide { last_event_date })
        }

        async fn save_sync(
            &self,
            client_id: AppId,
            sync: &SyncSave,
        ) -> Result<(), InfrastructureError> {
            self.0
                .query("UPDATE type::thing('client_sync', $client_id) REPLACE { data: $data }")
                .bind(("client_id", client_id))
                .bind(("data", serde_json::to_string(sync).unwrap()))
                .await?
                .check()?;

            Ok(())
        }

        async fn save_sync_error(
            &self,
            client_id: AppId,
            error: &SyncError,
        ) -> Result<(), InfrastructureError> {
            self.0
                .query("UPDATE type::thing('client_sync', $client_id) SET error=$error")
                .bind(("client_id", client_id))
                .bind(("error", error))
                .await?
                .check()?;

            Ok(())
        }

        async fn save_changes(
            &self,
            data: &SyncContextData,
        ) -> Result<(), AppError<UniqueSaveError>> {
            save_changes(&self.0, data).await
        }

        async fn get_context_data(&self) -> Result<SyncContextData, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT * FROM currency")
                .query("SELECT * FROM item_tag")
                .query("SELECT * FROM actor")
                .query("SELECT * FROM wallet")
                .await
                .catch_infra()?;

            #[derive(serde::Deserialize)]
            struct SurrealWallet {
                pub currency_id: EntityKey<monee_core::CurrencyId>,
                pub name: WalletName,
                pub description: String,
            }

            impl From<SurrealWallet> for Wallet {
                fn from(value: SurrealWallet) -> Self {
                    Wallet {
                        currency_id: value.currency_id.0,
                        name: value.name,
                        description: value.description,
                    }
                }
            }

            let currencies: Vec<Entity<CurrencyId, Currency>> = response.take(0)?;
            let items: Vec<Entity<ItemTagId, ItemTag>> = response.take(1)?;
            let actors: Vec<Entity<ActorId, Actor>> = response.take(2)?;
            let wallets: Vec<Entity<WalletId, SurrealWallet>> = response.take(3)?;

            Ok(SyncContextData {
                currencies: currencies.into_iter().map(Entity::into).collect(),
                items: items.into_iter().map(Entity::into).collect(),
                actors: actors.into_iter().map(Entity::into).collect(),
                wallets: wallets.into_iter().map(|e| (e.0, e.1.into())).collect(),
            })
        }
    }

    pub async fn save_changes(
        con: &Connection,
        data: &SyncContextData,
    ) -> Result<(), AppError<UniqueSaveError>> {
        let mut query = con.query(BeginStatement::default());

        for (id, currency) in data.currencies.iter() {
            query = query
                .query("UPDATE type::thing('currency', $currency_id) SET name = $name, symbol = $symbol, code = $code")
                .bind(("currency_id", id))
                .bind(currency);
        }

        for (id, item) in data.items.iter() {
            query = query
                .query("UPDATE type::thing('item_tag', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", item));
        }

        for (id, actor) in data.actors.iter() {
            query = query
                .query("UPDATE type::thing('actor', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", actor));
        }

        for (id, wallet) in data.wallets.iter() {
            query = query
                .query("UPDATE type::thing('wallet', $id) SET currency_id = type::thing('currency', $currency_id), name = $name, description = $description")
                .bind(("id", id))
                .bind(("currency_id", &wallet.currency_id))
                .bind(("name", &wallet.name))
                .bind(("description", &wallet.description));
        }

        query
            .query(CommitStatement::default())
            .await
            .catch_infra()?
            .check()
            .catch_app()?;

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn it_saves_items() {
            use super::*;
            use cream::context::Context;
            use monee_core::ItemTagId;
            use monee_types::backoffice::item_tags::item_tag::ItemTag;

            let con = crate::shared::infrastructure::database::connect()
                .await
                .unwrap();
            let ctx = DbContext::new(con);
            let data = SyncContextData {
                currencies: vec![],
                items: vec![(
                    ItemTagId::default(),
                    ItemTag {
                        name: "test".parse().unwrap(),
                    },
                )],
                actors: vec![],
                wallets: vec![],
            };

            let repo: SurrealRepository = ctx.provide();
            repo.save_changes(&data).await.unwrap();

            let data = repo.get_context_data().await.unwrap();
            assert_eq!(data.items.len(), 1);
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn saves_wallets() {
            use super::*;
            use cream::context::Context;
            use monee_core::CurrencyId;
            use monee_types::backoffice::{
                currencies::currency::Currency, wallets::wallet::Wallet,
            };

            let con = crate::shared::infrastructure::database::connect()
                .await
                .unwrap();
            let ctx = DbContext::new(con);
            let repo: SurrealRepository = ctx.provide();

            let currency_id = CurrencyId::default();

            repo.save_changes(&SyncContextData {
                currencies: vec![(
                    currency_id,
                    Currency {
                        name: "sol".to_owned().into(),
                        symbol: "S/".parse().unwrap(),
                        code: "PEN".parse().unwrap(),
                    },
                )],
                items: vec![],
                actors: vec![],
                wallets: vec![(
                    WalletId::default(),
                    Wallet {
                        currency_id,
                        name: "mine".parse().unwrap(),
                        description: "".to_owned(),
                    },
                )],
            })
            .await
            .unwrap();

            let data = repo.get_context_data().await.unwrap();
            assert_eq!(data.wallets.len(), 1);
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn saves_twice() {
            use super::*;
            use cream::context::Context;
            use monee_core::{CurrencyId, WalletId};
            use monee_types::backoffice::{
                currencies::currency::Currency, wallets::wallet::Wallet,
            };

            let con = crate::shared::infrastructure::database::connect()
                .await
                .unwrap();
            let ctx = DbContext::new(con);
            let repo: SurrealRepository = ctx.provide();

            let currency_id = CurrencyId::default();
            let wallet_id = WalletId::default();
            let save = || async {
                repo.save_changes(&SyncContextData {
                    currencies: vec![(
                        currency_id,
                        Currency {
                            name: "sol".to_owned().into(),
                            symbol: "S/".parse().unwrap(),
                            code: "PEN".parse().unwrap(),
                        },
                    )],
                    items: vec![],
                    actors: vec![],
                    wallets: vec![(
                        wallet_id,
                        Wallet {
                            currency_id,
                            name: "mine".parse().unwrap(),
                            description: "".to_owned(),
                        },
                    )],
                })
                .await
                .unwrap();
            };

            save().await;
            save().await;
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn save_multiple() {
            use super::*;
            use cream::context::Context;
            use monee_core::CurrencyId;
            use monee_types::backoffice::{
                currencies::currency::Currency, wallets::wallet::Wallet,
            };

            let con = crate::shared::infrastructure::database::connect()
                .await
                .unwrap();
            let ctx = DbContext::new(con);
            let repo: SurrealRepository = ctx.provide();

            let currency_id1 = CurrencyId::default();
            let currency_id2 = CurrencyId::default();

            repo.save_changes(&SyncContextData {
                currencies: vec![(
                    currency_id1,
                    Currency {
                        name: "sol".to_owned().into(),
                        symbol: "S/".parse().unwrap(),
                        code: "PEN".parse().unwrap(),
                    },
                ),
                (
                    currency_id2,
                    Currency {
                        name: "dollar".to_owned().into(),
                        symbol: "$".parse().unwrap(),
                        code: "USD".parse().unwrap(),
                    },
                )],
                items: vec![],
                actors: vec![],
                wallets: vec![(
                    WalletId::default(),
                    Wallet {
                        currency_id: currency_id1,
                        name: "mine".parse().unwrap(),
                        description: "".to_owned(),
                    },
                ), 
                (
                    WalletId::default(),
                    Wallet {
                        currency_id: currency_id2,
                        name: "othermine".parse().unwrap(),
                        description: "".to_owned(),
                    },
                )],
            })
            .await
            .unwrap();

            let data = repo.get_context_data().await.unwrap();
            assert_eq!(data.currencies.len(), 2, "Currencies");
            assert_eq!(data.wallets.len(), 2, "Wallets");
        }
    }
}
