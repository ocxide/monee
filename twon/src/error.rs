#[derive(Debug, thiserror::Error)]
pub enum SnapshotOptError {
    #[error(transparent)]
    Database(#[from] surrealdb::Error),

    #[error(transparent)]
    SnapshotApply(#[from] twon_core::Error),

    #[error(transparent)]
    Write(#[from] std::io::Error),

    #[error(transparent)]
    Read(#[from] crate::snapshot_io::ReadError),
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

impl From<SnapshotReadError> for SnapshotOptError {
    fn from(value: SnapshotReadError) -> Self {
        match value {
            SnapshotReadError::Database(error) => Self::Database(error),
            SnapshotReadError::SnapshotApply(error) => Self::SnapshotApply(error),
            SnapshotReadError::Read(error) => Self::Read(error),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotReadError {
    #[error(transparent)]
    Database(#[from] surrealdb::Error),

    #[error(transparent)]
    SnapshotApply(#[from] twon_core::Error),

    #[error(transparent)]
    Read(#[from] crate::snapshot_io::ReadError),
}

