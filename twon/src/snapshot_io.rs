pub use read_in::{do_read, read, Error as ReadError, JsonDecodeError};

use std::fs;

use crate::create_local_path;

const SNAPSHOT_FILENAME: &str = "snapshot.json";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnapshotMetadata {
    pub created_at: crate::date::Datetime,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnapshotEntry {
    pub metadata: SnapshotMetadata,
    pub snapshot: twon_core::Snapshot,
}

pub(crate) async fn write(snapshot: twon_core::Snapshot) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || do_write(snapshot))
        .await
        .expect("To join write task")
}

pub(crate) fn do_write(snapshot: twon_core::Snapshot) -> std::io::Result<()> {
    let entry = SnapshotEntry {
        snapshot,
        metadata: SnapshotMetadata {
            created_at: crate::date::Timezone::now(),
        },
    };

    let path = create_local_path().join(SNAPSHOT_FILENAME);
    let mut file = fs::File::options().truncate(true).open(path)?;
    serde_json::to_writer(&mut file, &entry).map_err(Into::into)
}

mod read_in {
    use std::path::PathBuf;

    use crate::create_local_path;

    use super::{SnapshotEntry, SnapshotMetadata, SNAPSHOT_FILENAME};

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Could not read snapshot file : {0}")]
        Io(#[from] std::io::Error),
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

    pub fn do_read() -> Result<SnapshotEntry, Error> {
        let path = create_local_path().join(SNAPSHOT_FILENAME);
        let buf = std::fs::read_to_string(&path)?;

        if buf.is_empty() {
            let snapshot = twon_core::Snapshot::default();
            return Ok(SnapshotEntry {
                metadata: SnapshotMetadata {
                    created_at: crate::date::Timezone::now(),
                },
                snapshot,
            });
        }

        match serde_json::from_str(&buf) {
            Ok(entry) => Ok(entry),
            Err(e) => Err(Error::Json(JsonDecodeError {
                error: e,
                json: buf,
                filename: path,
            })),
        }
    }

    pub async fn read() -> Result<SnapshotEntry, Error> {
        tokio::task::spawn_blocking(do_read)
            .await
            .expect("To join read task")
    }
}

