pub mod list_wallets {
    pub use crate::error::SnapshotReadError as Error;

    pub struct WalletRow {
        pub id: twon_core::WalletId,
        pub name: Option<String>,
        pub currency: twon_core::CurrencyId,
        pub balance: twon_core::Amount,
    }

    #[derive(serde::Deserialize)]
    pub struct WalletSelect {
        #[serde(with = "crate::sql_id::string")]
        pub id: twon_core::WalletId,
        pub name: Option<String>,
    }

    pub async fn run(
        connection: &crate::database::Connection,
    ) -> Result<Vec<WalletRow>, Error> {
        let snapshot_fut = tokio::task::spawn_blocking(move || {
            let mut snapshot_io = crate::snapshot_io::SnapshotIO::new();
            snapshot_io.read()
        });

        let metadatas = async move {
            let result: Result<Vec<WalletSelect>, _> =
                connection.select("wallet_metadata").await;
            result
        };

        let (join, metadatas) = tokio::join!(snapshot_fut, metadatas);

        let snapshot_entry = join.expect("To join read task")?;
        let metadatas = metadatas?;

        let wallets = snapshot_entry
            .snapshot
            .wallets
            .into_iter()
            .map(|(id, v)| WalletRow {
                id,
                currency: v.currency,
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

pub mod create_wallet {
    use surrealdb::sql::{self, Thing};
    use twon_core::WalletId;

    use crate::snapshot_io;

    pub use crate::error::SnapshotOptError as Error;

    pub async fn run(
        connection: &crate::database::Connection,
        currency_id: twon_core::CurrencyId,
        name: Option<String>,
    ) -> Result<WalletId, Error> {
        let wallet_id = WalletId::new();

        let mut snapshot_entry = tokio::task::spawn_blocking(move || {
            let mut snapshot_io = crate::snapshot_io::SnapshotIO::new();
            snapshot_io.read()
        })
        .await
        .expect("To join read task")?;

        let event = twon_core::Event::CreateWallet {
            wallet_id,
            currency: currency_id,
        };
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

        tokio::task::spawn_blocking(move || {
            let mut snapshot_io = snapshot_io::SnapshotIO::new();
            snapshot_io.write(snapshot_entry.snapshot)
        })
        .await
        .expect("To join write task")?;

        Ok(wallet_id)
    }
}

