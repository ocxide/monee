mod sql_id;

pub mod actions;
pub mod database;
pub mod error;
pub mod log;
pub mod ops;

pub mod procedures {
    pub struct BalanceRegister {
        pub wallet_id: twon_core::WalletId,
        pub amount: twon_core::Amount,
    }

    #[derive(serde::Serialize)]
    pub struct CreateProcedure {
        pub description: Option<String>,
    }

    pub enum CreateProcedureType {
        BalanceRegister {},
    }

    /// Stateless mirror of CreateProcedureType
    #[derive(serde::Serialize)]
    pub enum ProcedureType {
        BalanceRegister,
    }

    pub async fn balance_register(
        connection: &crate::database::Connection,
        procedure: CreateProcedure,
        kind: BalanceRegister,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let crate::snapshot_io::SnapshotEntry { mut snapshot, .. } =
            tokio::task::spawn_blocking(move || {
                let mut snapshot_io = crate::snapshot_io::SnapshotIO::new();
                snapshot_io.read()
            })
            .await
            .expect("to join read task")?;

        let event = twon_core::Event::Wallet(twon_core::WalletEvent::Deposit {
            wallet_id: kind.wallet_id,
            amount: kind.amount,
        });
        snapshot.apply(event.clone())?;

        let mut response = connection
            .query("CREATE procedure SET description = $description, type = $type RETURN id")
            .bind(procedure)
            .bind(("type", ProcedureType::BalanceRegister))
            .await?
            .check()?;

        let procedure_id: surrealdb::sql::Thing = response
            .take::<Vec<_>>("id")?
            .into_iter()
            .next()
            .expect("to get id");

        let mut response = connection
            .query("CREATE event CONTENT $data RETURN id")
            .bind(("data", event))
            .await?
            .check()?;

        let event_id: surrealdb::sql::Thing = response
            .take::<Vec<_>>("id")?
            .into_iter()
            .next()
            .expect("to get id");

        connection
            .query("RELATE $procedure->generated->$event")
            .bind(("procedure", procedure_id))
            .bind(("event", event_id))
            .await?
            .check()?;

        tokio::task::spawn_blocking(move || {
            let mut snapshot_io = crate::snapshot_io::SnapshotIO::new();
            snapshot_io.write(snapshot)
        })
        .await
        .expect("to join write task")?;

        Ok(())
    }
}

use std::{fs, path::PathBuf};

pub use database::connect;
pub use snapshot_io::SnapshotIO;

pub type Datetime = chrono::DateTime<chrono::Utc>;
pub type Timezone = chrono::Utc;

fn create_local_path() -> PathBuf {
    let share_dir = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local/share"))
        })
        .expect("To get share directory");
    let path = share_dir.join("twon");

    fs::create_dir_all(&path).expect("To create twon data directory");
    path
}

pub mod snapshot_io {
    use std::{
        fs,
        io::{self, Seek},
    };

    use crate::create_local_path;

    pub struct SnapshotIO(std::fs::File);

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SnapshotMetadata {
        pub created_at: crate::Datetime,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SnapshotEntry {
        pub metadata: SnapshotMetadata,
        pub snapshot: twon_core::Snapshot,
    }

    impl Default for SnapshotIO {
        fn default() -> Self {
            Self::new()
        }
    }

    pub mod read {
        use std::{
            io::{self, Read},
            path::PathBuf,
        };

        use super::{SnapshotEntry, SnapshotIO, SnapshotMetadata};

        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error("Could not read snapshot file : {0}")]
            Io(io::Error),
            #[error("Could not decode snapshot file : {0}")]
            Json(JsonDecodeError),
        }

        #[derive(Debug, thiserror::Error)]
        #[error("{error}")]
        pub struct JsonDecodeError {
            pub error: serde_json::Error,
            pub json: String,
            pub filename: PathBuf,
        }

        impl SnapshotIO {
            pub fn read(&mut self) -> Result<SnapshotEntry, Error> {
                let mut buf = String::new();
                self.0.read_to_string(&mut buf).map_err(Error::Io)?;

                if buf.is_empty() {
                    let snapshot = twon_core::Snapshot::default();
                    return Ok(SnapshotEntry {
                        metadata: SnapshotMetadata {
                            created_at: crate::Timezone::now(),
                        },
                        snapshot,
                    });
                }

                match serde_json::from_str(&buf) {
                    Ok(entry) => Ok(entry),
                    Err(e) => Err(Error::Json(JsonDecodeError {
                        error: e,
                        json: buf,
                        filename: Self::create_snapshot_path(),
                    })),
                }
            }
        }
    }

    impl SnapshotIO {
        fn create_snapshot_path() -> std::path::PathBuf {
            create_local_path().join("snapshot.json")
        }

        pub fn new() -> Self {
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true);

            Self::open(&opts)
        }

        pub fn open(options: &fs::OpenOptions) -> Self {
            let file = options
                .open(Self::create_snapshot_path())
                .expect("To open snapshot file");

            Self(file)
        }

        pub fn write(&mut self, snapshot: twon_core::Snapshot) -> io::Result<()> {
            self.0.set_len(0)?;
            self.0.rewind()?;

            let entry = SnapshotEntry {
                snapshot,
                metadata: SnapshotMetadata {
                    created_at: crate::Timezone::now(),
                },
            };

            serde_json::to_writer(&mut self.0, &entry).map_err(Into::into)
        }
    }
}
