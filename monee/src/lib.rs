mod sql_id;

pub mod error;
pub mod log;
pub mod ops;

// pub mod actions;
// pub mod procedures;

pub mod snapshot_io;

use std::{fs, path::PathBuf};

pub mod date {
    pub type Datetime = chrono::DateTime<chrono::Utc>;
    pub use chrono::Utc as Timezone;
}

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
    let path = share_dir.join("monee");

    fs::create_dir_all(&path).expect("To create monee data directory");
    path
}

// TODO remove
pub use crate::shared::infrastructure::database::Entity;

pub mod backoffice;
pub mod reports;
pub mod shared;
