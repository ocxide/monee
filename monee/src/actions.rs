pub mod item_tags;

pub mod snapshopts {
    pub mod show {
        use std::{collections::HashMap, future::IntoFuture, rc::Rc};

        pub struct SnapshotShow {
            pub wallets: Vec<(monee_core::WalletId, WalletShow)>,
            pub in_debts: Vec<(monee_core::DebtId, DebtShow)>,
            pub out_debts: Vec<(monee_core::DebtId, DebtShow)>,
        }

        pub struct WalletShow {
            pub money: monee_core::MoneyStorage,
            pub metadata: monee_core::metadata::WalletMetadata,
            pub currency: Option<Rc<monee_core::currency::Currency>>,
        }

        pub struct DebtShow {
            pub money: monee_core::MoneyStorage,
            pub currency: Option<Rc<monee_core::currency::Currency>>,
            pub actor: Vec<Rc<monee_core::actor::Actor>>,
        }

        #[derive(serde::Deserialize)]
        struct DebtActors {
            debt_id: monee_core::DebtId,
            #[serde(with = "crate::sql_id::string_vec")]
            actors: Vec<monee_core::actor::ActorId>,
        }

        async fn get_debt_actors(
            connection: &crate::database::Connection,
            debt_relation: &'static str,
            group: &'static str,
        ) -> Result<Vec<DebtActors>, crate::database::Error> {
            let mut response = connection
                .query(format!("SELECT <-generated<-procedure<-{debt_relation}<-actor as actors, debt_id FROM event WHERE group = $group AND type = 'incur'"))
                .bind(("group", group)).await?.check()?;

            response.take(0)
        }

        pub async fn run(
            connection: &crate::database::Connection,
        ) -> Result<SnapshotShow, crate::error::SnapshotReadError> {
            let crate::snapshot_io::SnapshotEntry { snapshot, .. } =
                crate::snapshot_io::read().await?;

            let currencies = crate::actions::currencies::list::run(connection);
            let actors = crate::actions::actors::list::run(connection);
            let metadatas = async {
                match connection
                    .query("SELECT * FROM wallet_metadata")
                    .into_future()
                    .await
                {
                    Ok(mut response) => response.take(0),
                    Err(e) => Err(e),
                }
            };
            let in_debt_actors = get_debt_actors(connection, "in_debt_on", "in_debt");
            let out_debt_actors = get_debt_actors(connection, "out_debt_on", "out_debt");

            let (currencies, actors, metadatas, in_debt_actors, out_debt_actors): (
                _,
                _,
                Vec<crate::Entity<monee_core::WalletId, monee_core::metadata::WalletMetadata>>,
                _,
                _,
            ) = tokio::try_join!(
                currencies,
                actors,
                metadatas,
                in_debt_actors,
                out_debt_actors
            )?;

            let currencies: HashMap<_, _> = currencies
                .into_iter()
                .map(|c| (c.0, Rc::new(c.1)))
                .collect();

            let actors: HashMap<_, _> = actors.into_iter().map(|a| (a.0, Rc::new(a.1))).collect();

            let wallets = snapshot
                .wallets
                .into_iter()
                .map(|(id, money)| {
                    let wallet = WalletShow {
                        currency: currencies.get(&money.currency).cloned(),
                        money,
                        metadata: metadatas.iter().find(|m| m.0 == id).expect("to get metadata").1.clone(),
                    };

                    (id, wallet)
                })
                .collect();

            let collect_debts = |debts: monee_core::MoneyRecord<monee_core::DebtId>,
                                 debt_actors: Vec<DebtActors>| {
                debts
                    .into_iter()
                    .map(|(id, money)| {
                        let debt = DebtShow {
                            currency: currencies.get(&money.currency).cloned(),
                            actor: debt_actors
                                .iter()
                                .find(|d| d.debt_id == id)
                                .map(|d| {
                                    d.actors
                                        .iter()
                                        .filter_map(|a| actors.get(a))
                                        .cloned()
                                        .collect()
                                })
                                .unwrap_or_default(),
                            money,
                        };

                        (id, debt)
                    })
                    .collect()
            };

            let in_debts = collect_debts(snapshot.in_debts, in_debt_actors);
            let out_debts = collect_debts(snapshot.out_debts, out_debt_actors);

            Ok(SnapshotShow {
                wallets,
                in_debts,
                out_debts,
            })
        }
    }
}

pub mod events {
    pub async fn add(
        connection: &crate::database::Connection,
        event: monee_core::Event,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let mut snapshot_entry = crate::snapshot_io::read().await?;
        snapshot_entry.snapshot.apply(event.clone())?;

        connection
            .query("CREATE event CONTENT $data")
            .bind(("data", event))
            .await?;

        crate::snapshot_io::write(snapshot_entry.snapshot).await?;
        Ok(())
    }

    #[derive(serde::Deserialize)]
    pub struct EventRow {
        #[serde(flatten)]
        pub event: monee_core::Event,
        pub created_at: crate::date::Datetime,
    }

    pub async fn list(
        connection: &crate::database::Connection,
    ) -> Result<Vec<EventRow>, crate::database::Error> {
        let events: Vec<EventRow> = connection.select("event").await?;
        Ok(events)
    }
}

pub mod debts {
    pub mod list {
        use core::panic;
        use std::future::IntoFuture;

        #[derive(serde::Deserialize)]
        struct QueryResult {
            actors: Vec<monee_core::actor::Actor>,
            debt_id: monee_core::DebtId,
        }

        pub struct DebtItem {
            pub debt_id: monee_core::DebtId,
            pub debt: monee_core::MoneyStorage,
            pub actors: Vec<monee_core::actor::Actor>,
            pub currency: Option<crate::actions::currencies::list::CurrencyRow>,
        }

        async fn run(
            connection: &crate::database::Connection,
            debt_relation: &'static str,
            group: &'static str,
            debts: monee_core::MoneyRecord<monee_core::DebtId>,
        ) -> Result<Vec<DebtItem>, crate::database::Error> {
            let debts_req = connection
                .query(format!("SELECT id, <-generated<-procedure<-{debt_relation}<-actor.* as actors, debt_id, currency_id FROM event WHERE group = $group AND type = 'incur'"))
                .bind(("group", group)).into_future();

            let currencies = crate::actions::currencies::list::run(connection);
            let (debts_req, currencies) = tokio::try_join!(debts_req, currencies)?;

            let mut response = debts_req.check()?;

            let results: Vec<QueryResult> = response.take(0)?;

            let response: Vec<_> = debts
                .into_iter()
                .map(|(debt_id, money)| {
                    let Some(debt) = results.iter().find(|r| r.debt_id == debt_id) else {
                        panic!("Missing debt {}", debt_id)
                    };

                    let currency = currencies.iter().find(|c| c.0 == money.currency).cloned();

                    DebtItem {
                        debt_id,
                        debt: money,
                        actors: debt.actors.clone(),
                        currency,
                    }
                })
                .collect();

            Ok(response)
        }

        pub async fn run_in(
            connection: &crate::database::Connection,
            debts: monee_core::MoneyRecord<monee_core::DebtId>,
        ) -> Result<Vec<DebtItem>, crate::database::Error> {
            let response = run(connection, "in_debt_on", "in_debt", debts).await?;
            Ok(response)
        }

        pub async fn run_out(
            connection: &crate::database::Connection,
            debts: monee_core::MoneyRecord<monee_core::DebtId>,
        ) -> Result<Vec<DebtItem>, crate::database::Error> {
            let response = run(connection, "out_debt_on", "out_debt", debts).await?;
            Ok(response)
        }
    }
}

pub mod actors;

pub mod currencies {
    pub mod from_code {
        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error("Currency code not found")]
            NotFound,
            #[error(transparent)]
            Database(#[from] crate::database::Error),
        }

        pub async fn run(
            connection: &crate::database::Connection,
            code: String,
        ) -> Result<monee_core::CurrencyId, Error> {
            #[derive(serde::Deserialize)]
            struct CurrencyIdSelect {
                #[serde(with = "crate::sql_id::string")]
                id: monee_core::CurrencyId,
            }

            let mut response = connection
                .query("SELECT id FROM currency WHERE code = $code")
                .bind(("code", code))
                .await?
                .check()?;

            let select: Vec<CurrencyIdSelect> = response.take(0)?;
            let Some(select) = select.into_iter().next() else {
                return Err(Error::NotFound);
            };

            Ok(select.id)
        }
    }

    pub mod check {
        pub async fn run(
            connection: &crate::database::Connection,
            id: monee_core::CurrencyId,
        ) -> Result<bool, crate::database::Error> {
            #[derive(serde::Deserialize)]
            struct Empty {}

            let mut response = connection
                .query("SELECT 1 FROM type::thing(currency, $id)")
                .bind(("id", id))
                .await?
                .check()?;

            let data: Option<Empty> = response.take(0)?;
            Ok(data.is_some())
        }
    }

    pub mod create {
        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error("Currency already exists")]
            AlreadyExists,
            #[error(transparent)]
            Database(#[from] crate::database::Error),
        }

        pub async fn run(
            connection: &crate::database::Connection,
            name: String,
            symbol: String,
            code: String,
        ) -> Result<monee_core::CurrencyId, Error> {
            let id = monee_core::CurrencyId::new();
            let response = connection
                .query(
                    "CREATE ONLY currency SET id=$id, name = $name, symbol = $symbol, code = $code",
                )
                .bind(("id", id))
                .bind(("name", name))
                .bind(("symbol", symbol))
                .bind(("code", code))
                .await?
                .check();

            match response {
                Err(
                    crate::database::Error::Api(surrealdb::error::Api::Query { .. })
                    | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                ) => Err(Error::AlreadyExists),
                Err(e) => Err(e.into()),
                Ok(_) => Ok(id),
            }
        }
    }

    pub mod list {
        use monee_core::currency;

        pub type CurrencyRow = crate::database::Entity<currency::CurrencyId, currency::Currency>;

        pub async fn run(
            connection: &crate::database::Connection,
        ) -> Result<Vec<CurrencyRow>, crate::database::Error> {
            let response: Vec<CurrencyRow> = connection.select("currency").await?;
            Ok(response)
        }
    }
}

pub mod wallets {
    pub mod list {
        pub use crate::error::SnapshotReadError as Error;

        pub struct WalletRow {
            pub id: monee_core::WalletId,
            pub name: Option<String>,
            pub currency: Option<crate::actions::currencies::list::CurrencyRow>,
            pub balance: monee_core::Amount,
        }

        pub async fn run(
            connection: &crate::database::Connection,
        ) -> Result<Vec<WalletRow>, Error> {
            let snapshot_fut = crate::snapshot_io::read();
            let metadatas = async move {
                let result: Result<
                    Vec<crate::Entity<monee_core::WalletId, monee_core::metadata::WalletMetadata>>,
                    _,
                > = connection.select("wallet_metadata").await;
                result
            };

            let curriencies = crate::actions::currencies::list::run(connection);

            let (snapshot_entry, metadatas, curriencies) =
                tokio::join!(snapshot_fut, metadatas, curriencies);

            let metadatas = metadatas?;
            let curriencies = curriencies?;
            let snapshot_entry = snapshot_entry?;

            let wallets = snapshot_entry
                .snapshot
                .wallets
                .into_iter()
                .map(|(id, v)| WalletRow {
                    id,
                    currency: curriencies.iter().find(|c| c.0 == v.currency).cloned(),
                    balance: v.balance,
                    name: metadatas
                        .iter()
                        .find(|w| w.0 == id)
                        .and_then(|crate::Entity(_, w)| w.name.clone()),
                })
                .collect();

            Ok(wallets)
        }
    }

    pub mod create {
        use monee_core::WalletId;
        use surrealdb::sql::{self, Thing};

        pub use crate::error::SnapshotOptError as Error;

        pub async fn run(
            connection: &crate::database::Connection,
            currency_id: monee_core::CurrencyId,
            name: Option<String>,
        ) -> Result<WalletId, Error> {
            let wallet_id = WalletId::new();

            let mut snapshot_entry = crate::snapshot_io::read().await?;
            let event = monee_core::Event::Wallet(monee_core::WalletEvent::Create {
                wallet_id,
                currency: currency_id,
            });
            snapshot_entry.snapshot.apply(event.clone())?;

            let wallet_resource = {
                let id = sql::Id::String(wallet_id.to_string());
                Thing::from(("wallet_metadata", id))
            };

            let response = connection
                .query(sql::statements::BeginStatement)
                .query(
                    "
CREATE event CONTENT $event;
CREATE $wallet_resource SET name = $name;",
                )
                .bind(("event", event))
                .bind(("wallet_resource", wallet_resource))
                .bind(("name", name))
                .query(sql::statements::CommitStatement)
                .await?;

            response.check()?;

            crate::snapshot_io::write(snapshot_entry.snapshot).await?;
            Ok(wallet_id)
        }
    }
}

