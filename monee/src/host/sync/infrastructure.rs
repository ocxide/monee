pub mod repository {
    use monee_core::{ActorId, CurrencyId, ItemTagId, Wallet, WalletId};
    use monee_types::backoffice::{
        actors::actor::Actor, currencies::currency::Currency, item_tags::item_tag::ItemTag,
    };
    use surrealdb::sql::statements::{BeginStatement, CommitStatement};

    use crate::{
        host::{
            client::domain::client_id::ClientId,
            sync::domain::{
                repository::Repository, sync_context_data::SyncContextData, sync_error::SyncError,
                sync_guide::SyncGuide, sync_save::SyncSave,
            },
        },
        iprelude::*,
        prelude::*,
        shared::{
            domain::{context::DbContext, date::Datetime, errors::UniqueSaveError},
            infrastructure::database::Entity,
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
            client_id: ClientId,
            sync: &SyncSave,
        ) -> Result<(), InfrastructureError> {
            self.0
                .query("UPSERT type::thing('client_sync', $client_id) REPLACE { data: $data }")
                .bind(("client_id", client_id))
                .bind(("data", serde_json::to_string(sync).unwrap()))
                .await?
                .check()?;

            Ok(())
        }

        async fn save_sync_error(
            &self,
            client_id: ClientId,
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
            let mut query = self.0.query(BeginStatement);

            for (id, currency) in data.currencies.iter() {
                query = query
                    .query("UPSERT type::thing('currency', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", currency));
            }

            for (id, item) in data.items.iter() {
                query = query
                    .query("UPSERT type::thing('item_tag', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", item));
            }

            for (id, actor) in data.actors.iter() {
                query = query
                    .query("UPSERT type::thing('actor', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", actor));
            }

            for (id, wallet) in data.wallets.iter() {
                query = query
                    .query("UPSERT type::thing('wallet', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", wallet));
            }

            query
                .query(CommitStatement)
                .await
                .catch_infra()?
                .check()
                .catch_app()?;

            Ok(())
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

            let currencies: Vec<Entity<CurrencyId, Currency>> = response.take(0)?;
            let items: Vec<Entity<ItemTagId, ItemTag>> = response.take(0)?;
            let actors: Vec<Entity<ActorId, Actor>> = response.take(0)?;
            let wallets: Vec<Entity<WalletId, Wallet>> = response.take(0)?;

            Ok(SyncContextData {
                currencies: currencies.into_iter().map(Entity::into).collect(),
                items: items.into_iter().map(Entity::into).collect(),
                actors: actors.into_iter().map(Entity::into).collect(),
                wallets: wallets.into_iter().map(Entity::into).collect(),
            })
        }
    }
}
