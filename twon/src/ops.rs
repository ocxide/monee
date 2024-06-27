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

