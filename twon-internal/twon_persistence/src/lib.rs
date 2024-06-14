use std::{fs, path::PathBuf};

pub use database::connect;
pub use snapshot_io::SnapshotIO;

pub mod database {
    use crate::create_local_path;
    use surrealdb::engine::local::Db;
    pub use surrealdb::Result;

    pub type Connection = surrealdb::Surreal<Db>;

    pub async fn connect() -> surrealdb::Result<Connection> {
        let path = create_local_path().join("twon.db");
        let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
            "file://{}",
            path.display()
        ))
        .await?;
        db.use_ns("twon").use_db("twon").await?;

        Ok(db)
    }

    #[derive(serde::Serialize)]
    pub struct EventRow {
        #[serde(flatten)]
        pub inner: twon_core::Event,
        pub created_at: u8,
    }

    #[derive(serde::Deserialize)]
    pub struct Record {}

    pub async fn add_event(
        connection: &Connection,
        event: twon_core::Event,
        created_at: u8,
    ) -> Result<()> {
        let row = EventRow {
            created_at,
            inner: event,
        };
        let _: Vec<Record> = connection.create("event").content(row).await?;

        Ok(())
    }
}

fn create_local_path() -> PathBuf {
    let home = std::env::var("HOME").expect("To read $HOME");
    let path = PathBuf::from(home).join(".local/share/twon/");

    fs::create_dir_all(&path).expect("To create snapshot directory");
    path
}

mod snapshot_io {
    use std::{
        fs,
        io::{self, Read, Seek},
    };

    use crate::create_local_path;

    pub struct SnapshotIO(std::fs::File);

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SnapshotMetadata {
        pub created_at: u8,
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

    impl SnapshotIO {
        pub fn new() -> Self {
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true);

            Self::open(&opts)
        }

        pub fn open(options: &fs::OpenOptions) -> Self {
            let path = create_local_path();
            let file = options
                .open(path.join("snapshot.json"))
                .expect("To open snapshot file");

            Self(file)
        }

        pub fn read(&mut self) -> io::Result<SnapshotEntry> {
            let mut buf = String::new();
            self.0.read_to_string(&mut buf)?;

            if buf.is_empty() {
                let snapshot = twon_core::Snapshot::default();
                return Ok(SnapshotEntry {
                    metadata: SnapshotMetadata { created_at: 0 },
                    snapshot,
                });
            }

            serde_json::from_str(&buf).map_err(Into::into)
        }

        pub fn write(&mut self, entry: SnapshotEntry) -> io::Result<()> {
            self.0.set_len(0)?;
            self.0.rewind()?;

            serde_json::to_writer(&mut self.0, &entry).map_err(Into::into)
        }
    }
}
