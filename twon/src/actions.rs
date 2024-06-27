pub mod events {
    pub async fn add(
        connection: &crate::database::Connection,
        event: twon_core::Event,
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
}

pub mod debts {
    pub mod list {
        use core::panic;
        use std::future::IntoFuture;

        #[derive(serde::Deserialize)]
        struct QueryResult {
            actors: Vec<twon_core::actor::Actor>,
            debt_id: twon_core::DebtId,
        }

        pub struct DebtItem {
            pub debt_id: twon_core::DebtId,
            pub debt: twon_core::MoneyStorage,
            pub actors: Vec<twon_core::actor::Actor>,
            pub currency: Option<crate::actions::currencies::list::CurrencyRow>,
        }

        async fn run(
            connection: &crate::database::Connection,
            debt_relation: &'static str,
            group: &'static str,
            debts: twon_core::MoneyRecord<twon_core::DebtId>,
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

                    let currency = currencies.iter().find(|c| c.id == money.currency).cloned();

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
            debts: twon_core::MoneyRecord<twon_core::DebtId>,
        ) -> Result<Vec<DebtItem>, crate::database::Error> {
            let response = run(connection, "in_debt_on", "in_debt", debts).await?;
            Ok(response)
        }

        pub async fn run_out(
            connection: &crate::database::Connection,
            debts: twon_core::MoneyRecord<twon_core::DebtId>,
        ) -> Result<Vec<DebtItem>, crate::database::Error> {
            let response = run(connection, "out_debt_on", "out_debt", debts).await?;
            Ok(response)
        }
    }
}

pub mod actors {
    pub mod list {
        #[derive(serde::Deserialize)]
        pub struct ActorRow {
            #[serde(with = "crate::sql_id::string")]
            pub id: twon_core::actor::ActorId,
            #[serde(flatten)]
            pub data: twon_core::actor::Actor,
        }

        pub async fn run(
            connection: &crate::database::Connection,
        ) -> Result<Vec<ActorRow>, crate::database::Error> {
            let mut response = connection.query("SELECT * FROM actor").await?.check()?;

            let actors: Vec<ActorRow> = response.take(0)?;
            Ok(actors)
        }
    }

    pub mod create {
        use twon_core::actor;

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error("Actor already exists")]
            AlreadyExists,
            #[error(transparent)]
            Database(#[from] crate::database::Error),
        }

        pub async fn run(
            connection: &crate::database::Connection,
            actor: actor::Actor,
        ) -> Result<actor::ActorId, Error> {
            let id = actor::ActorId::new();
            println!("Creating actor id: {:?}", id);

            let result = connection
                .query("CREATE type::thing('actor', $id) CONTENT $data")
                .bind(("id", id))
                .bind(("data", actor))
                .await?
                .check();

            match result {
                Err(
                    crate::database::Error::Api(surrealdb::error::Api::Query { .. })
                    | surrealdb::Error::Db(surrealdb::error::Db::IndexExists { .. }),
                ) => Err(Error::AlreadyExists),
                Err(e) => Err(e.into()),
                Ok(_) => Ok(id),
            }
        }
    }
}

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
        ) -> Result<twon_core::CurrencyId, Error> {
            #[derive(serde::Deserialize)]
            struct CurrencyIdSelect {
                #[serde(with = "crate::sql_id::string")]
                id: twon_core::CurrencyId,
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
            id: twon_core::CurrencyId,
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
        ) -> Result<twon_core::CurrencyId, Error> {
            let id = twon_core::CurrencyId::new();
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
        #[derive(serde::Deserialize, Clone)]
        pub struct CurrencyRow {
            #[serde(with = "crate::sql_id::string")]
            pub id: twon_core::CurrencyId,
            pub name: String,
            pub symbol: String,
            pub code: String,
        }

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
            pub id: twon_core::WalletId,
            pub name: Option<String>,
            pub currency: Option<crate::actions::currencies::list::CurrencyRow>,
            pub balance: twon_core::Amount,
        }

        #[derive(serde::Deserialize)]
        struct WalletSelect {
            #[serde(with = "crate::sql_id::string")]
            pub id: twon_core::WalletId,
            pub name: Option<String>,
        }

        pub async fn run(
            connection: &crate::database::Connection,
        ) -> Result<Vec<WalletRow>, Error> {
            let snapshot_fut = crate::snapshot_io::read();
            let metadatas = async move {
                let result: Result<Vec<WalletSelect>, _> =
                    connection.select("wallet_metadata").await;
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
                    currency: curriencies.iter().find(|c| c.id == v.currency).cloned(),
                    balance: v.balance,
                    name: metadatas
                        .iter()
                        .find(|w| w.id == id)
                        .and_then(|w| w.name.clone()),
                })
                .collect();

            Ok(wallets)
        }
    }

    pub mod create {
        use surrealdb::sql::{self, Thing};
        use twon_core::WalletId;

        pub use crate::error::SnapshotOptError as Error;

        pub async fn run(
            connection: &crate::database::Connection,
            currency_id: twon_core::CurrencyId,
            name: Option<String>,
        ) -> Result<WalletId, Error> {
            let wallet_id = WalletId::new();

            let mut snapshot_entry = crate::snapshot_io::read().await?;
            let event = twon_core::Event::Wallet(twon_core::WalletEvent::Create {
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
