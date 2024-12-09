pub mod repository {
    use monee_core::{ActorId, CurrencyId, ItemTagId, Wallet, WalletId};
    use surrealdb::sql::statements::{BeginStatement, CommitStatement};

    use crate::{
        backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
            item_tags::domain::item_tag::ItemTag,
        },
        host::{
            client::domain::client_id::ClientId,
            sync::domain::{
                repository::Repository,
                sync_data::{Entry, SyncData},
                sync_error::SyncError,
                sync_guide::SyncGuide,
            },
        },
        iprelude::*,
        prelude::*,
        shared::domain::{context::DbContext, date::Datetime, errors::UniqueSaveError},
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
            sync: &SyncData,
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
            currencies: &[Entry<CurrencyId, Currency>],
            items: &[Entry<ItemTagId, ItemTag>],
            actors: &[Entry<ActorId, Actor>],
            wallets: &[Entry<WalletId, Wallet>],
        ) -> Result<(), AppError<UniqueSaveError>> {
            let mut query = self.0.query(BeginStatement);

            for Entry { id, data: currency } in currencies {
                query = query
                    .query("UPSERT type::thing('currency', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", currency));
            }

            for Entry { id, data: item } in items {
                query = query
                    .query("UPSERT type::thing('item_tag', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", item));
            }

            for Entry { id, data: actor } in actors {
                query = query
                    .query("UPSERT type::thing('actor', $id) CONTENT $data")
                    .bind(("id", id))
                    .bind(("data", actor));
            }

            for Entry { id, data: wallet } in wallets {
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
    }
}
