pub mod repository {
    use cream::context::FromContext;
    use monee_core::WalletId;

    use crate::{
        backoffice::wallets::domain::{
            repository::{Repository, UpdateError},
            wallet::Wallet,
            wallet_name::WalletName,
        },
        iprelude::{CatchApp, CatchInfra, MapResponse},
        shared::{
            domain::{context::DbContext, errors::UniqueSaveError},
            infrastructure::{
                database::{Connection, EntityKey},
                errors::{AppError, InfrastructureError},
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
                .await
                .catch_infra()?
                .check();

            result.catch_app().map_response()
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
                .catch_infra()?;

            let wallet_id: Option<EntityKey<WalletId>> = response.take(0).catch_infra()?;
            Ok(wallet_id.map(|w| w.0))
        }
    }

    #[cfg(all(test, feature = "db_test"))]
    mod test {
        use monee_core::CurrencyId;
        use cream::context::Context;

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

