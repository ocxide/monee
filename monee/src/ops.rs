pub mod sync {
    pub use crate::error::SnapshotOptError as Error;

    pub async fn sync() -> Result<(), Error> {
        let entry = crate::snapshot_io::read().await?;

        let snapshot = entry.snapshot;
        let min_date = entry.metadata.created_at;
        crate::ops::build::build(snapshot, min_date).await?;

        Ok(())
    }
}

pub mod build {
    use crate::database;

    pub use crate::error::SnapshotWriteError as Error;

    const STEP_SIZE: usize = 1000;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(Clone, Debug)]
    pub struct EventRow {
        pub created_at: chrono::DateTime<chrono::Utc>,
        #[serde(flatten)]
        pub event: monee_core::Event,
    }

    pub(crate) async fn build(
        mut snapshot: monee_core::Snapshot,
        mut min_date: crate::date::Datetime,
    ) -> Result<(), Error> {
        let connection = database::connect().await?;

        loop {
            let mut result = connection
                .query(
                    "SELECT * FROM event WHERE created_at > $min ORDER BY created_at LIMIT $limit",
                )
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

        crate::snapshot_io::write(snapshot).await?;
        Ok(())
    }

    pub(crate) struct EventsStream<'c> {
        last_date: crate::date::Datetime,
        connection: &'c crate::database::Connection,
    }

    impl<'c> EventsStream<'c> {
        pub fn new(
            connection: &'c crate::database::Connection,
            last_date: crate::date::Datetime,
        ) -> Self {
            Self {
                last_date,
                connection,
            }
        }

        pub async fn next(&mut self) -> Result<Option<Vec<EventRow>>, crate::database::Error> {
            let mut result = self
                .connection
                .query(
                    "SELECT * FROM event WHERE created_at > <datetime>$min ORDER BY created_at LIMIT $limit",
                )
                .bind(("min", self.last_date))
                .bind(("limit", STEP_SIZE)).await?.check()?;

            let events: Vec<EventRow> = result.take(0)?;
            let Some(last) = events.last() else {
                return Ok(None);
            };

            self.last_date = last.created_at;
            Ok(Some(events))
        }
    }
}

pub mod rebuild {
    use super::build::{EventRow, EventsStream};

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Apply error")]
        Apply(Box<ApplyError>),

        #[error(transparent)]
        Database(#[from] crate::database::Error),

        #[error(transparent)]
        Write(#[from] std::io::Error),
    }

    #[derive(Debug)]
    pub struct ApplyError {
        pub snapshot: monee_core::Snapshot,
        pub previous: Vec<EventRow>,
        pub at: EventRow,
        pub next: Vec<EventRow>,
        pub error: monee_core::Error,
    }

    pub async fn rebuild() -> Result<(), Error> {
        let db = crate::database::connect().await?;
        let mut stream = EventsStream::new(&db, crate::date::Datetime::UNIX_EPOCH);

        let mut snapshot = monee_core::Snapshot::default();

        while let Some(events) = stream.next().await? {
            for (i, event) in events.iter().enumerate() {
                let Err(e) = snapshot.apply(event.event.clone()) else {
                    continue;
                };

                let previous = events[i.saturating_sub(3)..i.saturating_sub(1)].to_vec();
                let next = {
                    let max_index = events.len().saturating_sub(1);
                    let start = std::cmp::min(i + 1, max_index);
                    let end = std::cmp::min(i + 3, max_index);
                    events[start..end].to_vec()
                };

                let error = ApplyError {
                    snapshot,
                    previous,
                    at: event.clone(),
                    next,
                    error: e,
                };

                return Err(Error::Apply(Box::new(error)));
            }
        }

        crate::snapshot_io::write(snapshot).await?;
        Ok(())
    }
}
