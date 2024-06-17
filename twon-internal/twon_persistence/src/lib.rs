use std::{fs, path::PathBuf};

pub use database::connect;
pub use snapshot_io::SnapshotIO;

pub type Datetime = chrono::DateTime<chrono::Utc>;
pub type Timezone = chrono::Utc;

pub mod log {
    use std::io::Write;

    const FILE: &str = "twon.log";

    fn write_error_log<E: std::error::Error>(error: E) {
        let path = crate::create_local_path().join(FILE);
        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(file) => file,
            Err(e) => {
                println!("WARNING - Unable to open log file: {}", e);
                return;
            }
        };

        let now = crate::Timezone::now();
        let result = writeln!(
            file,
            "ERROR {} - {} {}:{} - {error:?}",
            now.format("%d/%m/%Y %H:%M"),
            file!(),
            line!(),
            column!()
        );

        if let Err(e) = result {
            println!("WARNING - Unable to write to log file: {}", e);
        }
    }

    pub fn database(error: surrealdb::Error) -> ! {
        write_error_log(error);
        panic!("Error: Database error, aborting...");
    }

    pub fn snapshot_read(error: std::io::Error) -> ! {
        write_error_log(error);
        panic!("Error: Snapshot read error, aborting...");
    }

    pub fn snapshot_write(error: std::io::Error) -> ! {
        write_error_log(error);
        panic!("Error: Snapshot write error, aborting...");
    }
}

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum SnapshotOptError {
        #[error(transparent)]
        Database(#[from] surrealdb::Error),

        #[error(transparent)]
        SnapshotApply(#[from] twon_core::Error),

        #[error(transparent)]
        Write(#[from] std::io::Error),

        #[error(transparent)]
        Read(#[from] crate::snapshot_io::read::Error),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum SnapshotWriteError {
        #[error(transparent)]
        Database(#[from] surrealdb::Error),

        #[error(transparent)]
        SnapshotApply(#[from] twon_core::Error),

        #[error(transparent)]
        Write(#[from] std::io::Error),
    }

    impl From<SnapshotWriteError> for SnapshotOptError {
        fn from(value: SnapshotWriteError) -> Self {
            match value {
                SnapshotWriteError::Database(error) => Self::Database(error),
                SnapshotWriteError::SnapshotApply(error) => Self::SnapshotApply(error),
                SnapshotWriteError::Write(error) => Self::Write(error),
            }
        }
    }
}

pub mod actions {
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
}

pub mod ops {
    pub mod sync {
        use crate::snapshot_io;

        pub use crate::error::SnapshotOptError as Error;

        pub async fn sync() -> Result<(), Error> {
            let entry = tokio::task::spawn_blocking(move || {
                let mut snapshot_io = snapshot_io::SnapshotIO::new();
                snapshot_io.read()
            })
            .await
            .expect("To join read task")?;

            let snapshot = entry.snapshot;
            let min_date = entry.metadata.created_at;

            crate::ops::build::build(snapshot, min_date).await?;

            Ok(())
        }
    }

    pub mod build {
        use crate::{database, snapshot_io};

        pub use crate::error::SnapshotWriteError as Error;

        const STEP_SIZE: usize = 1000;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub struct EventRow {
            pub created_at: chrono::DateTime<chrono::Utc>,
            #[serde(flatten)]
            pub event: twon_core::Event,
        }

        pub(crate) async fn build(
            mut snapshot: twon_core::Snapshot,
            mut min_date: crate::Datetime,
        ) -> Result<(), Error> {
            let connection = database::connect().await?;

            loop {
                let mut result = connection
                    .query("SELECT * FROM event WHERE created_at > $min ORDER BY created_at LIMIT $limit")
                    .bind(("min", min_date))
                    .bind(("limit", STEP_SIZE))
                    .await?
                    .check()?;

                let events: Vec<EventRow> = result.take(0)?;
                let Some(last) = events.last() else {
                    break;
                };

                min_date = last.created_at;

                for event in events {
                    snapshot.apply(event.event)?;
                }
            }

            tokio::task::spawn_blocking(move || {
                let mut snapshot_io = snapshot_io::SnapshotIO::new();

                snapshot_io.write(snapshot)
            })
            .await
            .expect("To join write task")?;

            Ok(())
        }

        pub async fn rebuild() -> Result<(), Error> {
            let snapshot = twon_core::Snapshot::default();
            let min_date = crate::Datetime::MIN_UTC;

            build(snapshot, min_date).await
        }
    }
}

pub mod database {
    use crate::create_local_path;
    use surrealdb::engine::local::Db;
    pub use surrealdb::Result;

    pub type Connection = surrealdb::Surreal<Db>;
    const DB_DIR: &str = "twon.db";

    async fn init(connection: &Connection) -> Result<()> {
        connection
            .query("DEFINE TABLE event")
            .query("DEFINE FIELD created_at ON event VALUE time::now()")
            .await?
            .check()?;

        connection
            .query("DEFINE TABLE wallet_metadata")
            .query("DEFINE FIELD id ON wallet_metadata TYPE int")
            .query("DEFINE FIELD name ON wallet_metadata TYPE option<string>")
            .await?
            .check()?;

        Ok(())
    }

    async fn setup(connection: &Connection) -> Result<()> {
        // Skip initialization if db exists
        match tokio::fs::try_exists(create_local_path().join(DB_DIR)).await {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(_) => {
                println!("WARNING: Failed to check if db exists");
            }
        };

        let result = init(connection).await;
        if result.is_err() {
            println!("WARNING: Failed to initialize db");
        }

        result
    }

    pub async fn connect() -> surrealdb::Result<Connection> {
        let path = create_local_path().join(DB_DIR);
        let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
            "file://{}",
            path.display()
        ))
        .await?;
        db.use_ns("twon").use_db("twon").await?;

        setup(&db).await?;

        Ok(db)
    }

    pub async fn add_event(connection: &Connection, event: twon_core::Event) -> Result<()> {
        connection
            .query("CREATE event CONTENT $data")
            .bind(("data", event))
            .await?
            .check()?;

        Ok(())
    }
}

fn create_local_path() -> PathBuf {
    let home = std::env::var("HOME").expect("To read $HOME");
    let path = PathBuf::from(home).join(".local/share/twon/");

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
