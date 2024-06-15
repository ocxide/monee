use std::{fs, path::PathBuf};

pub use database::connect;
pub use snapshot_io::SnapshotIO;

pub type Datetime = chrono::DateTime<chrono::Utc>;
pub type Timezone = chrono::Utc;

pub mod ops {
    pub mod build {
        use crate::{database, snapshot_io};

        pub enum Error {
            Database,
            SnapshotApply(twon_core::Error),
            Write,
        }

        impl From<surrealdb::Error> for Error {
            fn from(_: surrealdb::Error) -> Self {
                Self::Database
            }
        }

        impl From<twon_core::Error> for Error {
            fn from(error: twon_core::Error) -> Self {
                Self::SnapshotApply(error)
            }
        }

        const STEP_SIZE: usize = 1000;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub struct EventRow {
            pub created_at: chrono::DateTime<chrono::Utc>,
            #[serde(flatten)]
            pub event: twon_core::Event,
        }

        pub async fn rebuild() -> Result<(), Error> {
            let connection = database::connect().await?;

            let mut min_date = chrono::DateTime::<chrono::Utc>::MIN_UTC;
            let mut snapshot = twon_core::Snapshot::default();

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

                snapshot_io.write(snapshot).unwrap();
            })
            .await
            .expect("To join write task");

            Ok(())
        }
    }
}

pub mod database {
    use crate::create_local_path;
    use surrealdb::engine::local::Db;
    pub use surrealdb::Result;

    pub type Connection = surrealdb::Surreal<Db>;

    async fn setup(connection: &Connection) -> Result<()> {
        connection
            .query("DEFINE TABLE event")
            .query("DEFINE FIELD created_at ON event VALUE time::now()")
            .await?
            .check()?;

        Ok(())
    }

    pub async fn connect() -> surrealdb::Result<Connection> {
        let path = create_local_path().join("twon.db");
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

    fs::create_dir_all(&path).expect("To create snapshot directory");
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

        pub enum Error {
            Io(io::Error),
            Json(JsonDecodeError),
        }

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
