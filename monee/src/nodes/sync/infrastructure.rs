pub mod repository {
    use crate::{
        backoffice::events::infrastructure::repository::SurrealMoneeEvent,
        host::sync::infrastructure::repository::save_changes,
        iprelude::*,
        nodes::sync::domain::repository::Repository,
        prelude::*,
        shared::{
            domain::context::DbContext,
            infrastructure::database::{Connection, Entity, EntityKey},
        },
    };
    use monee_core::{CurrencyId, ItemTagId};
    use monee_types::{
        backoffice::{
            actors::actor::Actor,
            currencies::currency::Currency,
            item_tags::item_tag::ItemTag,
            wallets::{wallet::Wallet, wallet_name::WalletName},
        },
        host::sync::node_changes::EventEntry,
        shared::{date::Datetime, errors::UniqueSaveError},
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(Connection);

    #[async_trait::async_trait]
    impl Repository for SurrealRepository {
        async fn truncate_events(&self) -> Result<(), InfrastructureError> {
            self.0.query("DELETE event").await?.check()?;
            Ok(())
        }

        async fn save_catalog(
            &self,
            data: &monee_types::host::sync::catalog::Catalog,
        ) -> Result<(), AppError<UniqueSaveError>> {
            save_changes(&self.0, data).await
        }

        async fn get_catalog(
            &self,
            changes: &monee_types::nodes::sync::changes_record::ChangesRecord,
        ) -> Result<monee_types::host::sync::catalog::Catalog, InfrastructureError> {
            let mut response = self
                .0
                .query("SELECT * FROM $currencies")
                .bind((
                    "currencies",
                    changes
                        .currencies
                        .iter()
                        .copied()
                        .map(EntityKey)
                        .collect::<Vec<_>>(),
                ))
                .query("SELECT * FROM $actors")
                .bind((
                    "actors",
                    changes
                        .actors
                        .iter()
                        .copied()
                        .map(EntityKey)
                        .collect::<Vec<_>>(),
                ))
                .query("SELECT * FROM $wallets")
                .bind((
                    "wallets",
                    changes
                        .wallets
                        .iter()
                        .copied()
                        .map(EntityKey)
                        .collect::<Vec<_>>(),
                ))
                .query("SELECT * FROM $items")
                .bind((
                    "items",
                    changes
                        .items
                        .iter()
                        .copied()
                        .map(EntityKey)
                        .collect::<Vec<_>>(),
                ))
                .await?;

            #[derive(serde::Deserialize)]
            struct SurrealWallet {
                pub currency_id: EntityKey<monee_core::CurrencyId>,
                pub name: WalletName,
                pub description: String,
            }

            let currencies: Vec<Entity<CurrencyId, Currency>> = response.take(0)?;
            let actors: Vec<Entity<monee_core::ActorId, Actor>> = response.take(1)?;
            let wallets: Vec<Entity<monee_core::WalletId, SurrealWallet>> = response.take(2)?;
            let items: Vec<Entity<ItemTagId, ItemTag>> = response.take(3)?;

            Ok(monee_types::host::sync::catalog::Catalog {
                currencies: currencies.into_iter().map(Into::into).collect(),
                actors: actors.into_iter().map(Into::into).collect(),
                wallets: wallets
                    .into_iter()
                    .map(Entity::into_inner)
                    .map(|(id, wallet)| {
                        (
                            id,
                            Wallet {
                                currency_id: wallet.currency_id.0,
                                name: wallet.name,
                                description: wallet.description,
                            },
                        )
                    })
                    .collect(),
                items: items.into_iter().map(Into::into).collect(),
            })
        }

        async fn get_events(
            &self,
            guide: monee_types::host::sync::sync_guide::SyncGuide,
        ) -> Result<Vec<EventEntry>, InfrastructureError> {
            #[derive(serde::Serialize, serde::Deserialize)]
            struct SurrealEventEntry {
                id: EntityKey<monee_core::EventId>,
                #[serde(flatten)]
                event: SurrealMoneeEvent,
                created_at: Datetime,
            }

            let mut response = self
                .0
                .query("SELECT * FROM event WHERE date > $date")
                .bind(("date", guide.last_event_date))
                .await?;

            let events: Vec<SurrealEventEntry> = response.take(0)?;
            Ok(events
                .into_iter()
                .map(|e| EventEntry {
                    id: e.id.0,
                    event: e.event.into(),
                    created_at: e.created_at,
                })
                .collect())
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(unused)]

        use monee_core::{ActorId, EventId, WalletId};
        use monee_types::{
            backoffice::{
                actors::actor_type::ActorType,
                events::event::{Buy, Event},
            },
            host::sync::catalog::Catalog,
            nodes::sync::changes_record::ChangesRecord,
        };

        use crate::{
            backoffice::events::domain::repository::Repository as EventsRepo,
            nodes::sync::domain::repository::Repository,
        };

        use super::*;

        async fn save_basic_catalog(repo: &SurrealRepository) -> Catalog {
            let item_tag_id = ItemTagId::default();
            let currency_id = CurrencyId::default();
            let actor_id = monee_core::ActorId::default();
            let wallet_id = monee_core::WalletId::default();

            let org_catalog = Catalog {
                items: vec![(
                    item_tag_id,
                    ItemTag {
                        name: "test".parse().unwrap(),
                    },
                )],
                currencies: vec![(
                    currency_id,
                    Currency {
                        name: "test".to_owned().into(),
                        symbol: "S$".parse().unwrap(),
                        code: "DON".parse().unwrap(),
                    },
                )],
                actors: vec![(
                    actor_id,
                    Actor {
                        name: "test".to_owned().into(),
                        actor_type: ActorType::Natural,
                        alias: None,
                    },
                )],
                wallets: vec![(
                    wallet_id,
                    Wallet {
                        currency_id,
                        name: "test".parse().unwrap(),
                        description: "test".into(),
                    },
                )],
            };

            repo.save_catalog(&org_catalog).await.unwrap();

            org_catalog
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn gets_catalog() {
            use crate::shared::infrastructure::database::connect;
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let repo: SurrealRepository = ctx.provide();

            let org_catalog = save_basic_catalog(&repo).await;
            repo.save_catalog(&org_catalog)
                .await
                .expect("should save catalog");

            let record = ChangesRecord {
                items: org_catalog.items.iter().map(|(id, _)| *id).collect(),
                currencies: org_catalog.currencies.iter().map(|(id, _)| *id).collect(),
                actors: org_catalog.actors.iter().map(|(id, _)| *id).collect(),
                wallets: org_catalog.wallets.iter().map(|(id, _)| *id).collect(),
            };
            let catalog = repo.get_catalog(&record).await.expect("should get catalog");

            assert_eq!(catalog, org_catalog, "catalog should be the same");
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn gets_events() {
            use crate::shared::infrastructure::database::connect;
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let back_repo: crate::backoffice::events::infrastructure::repository::SurrealRepository =
                ctx.provide();

            back_repo
                .add(
                    EventId::default(),
                    Event::Buy(Buy {
                        item: ItemTagId::default(),
                        actors: vec![ActorId::default()].into(),
                        wallet_id: WalletId::default(),
                        amount: "1.00".parse().unwrap(),
                    }),
                )
                .await
                .expect("should add event");

            let guide = monee_types::host::sync::sync_guide::SyncGuide {
                last_event_date: None,
            };
            let repo: SurrealRepository = ctx.provide();
            let events = repo.get_events(guide).await.expect("should get events");

            assert_eq!(events.len(), 1, "should have one event");
            assert!(
                matches!(events[0].event, Event::Buy(_)),
                "event should be buy"
            );
        }

        #[cfg(feature = "db_test")]
        #[tokio::test]
        async fn gets_events_with_date() {
            use crate::shared::infrastructure::database::connect;
            let db = connect().await.unwrap();
            let ctx = DbContext::new(db);

            let back_repo: crate::backoffice::events::infrastructure::repository::SurrealRepository =
            ctx.provide();

            back_repo
                .add(
                    EventId::default(),
                    Event::Buy(Buy {
                        item: ItemTagId::default(),
                        actors: vec![ActorId::default()].into(),
                        wallet_id: WalletId::default(),
                        amount: "1.00".parse().unwrap(),
                    }),
                )
                .await
                .expect("should add event");

            let guide = monee_types::host::sync::sync_guide::SyncGuide {
                last_event_date: Some(Datetime::MIN_UTC),
            };
            let repo: SurrealRepository = ctx.provide();
            let events = repo.get_events(guide).await.expect("should get events");

            assert_eq!(events.len(), 1, "should have one event");
            assert!(
                matches!(events[0].event, Event::Buy(_)),
                "event should be buy"
            );
        }
    }
}
