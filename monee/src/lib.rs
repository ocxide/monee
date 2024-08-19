pub mod date {
    pub type Datetime = chrono::DateTime<chrono::Utc>;
    pub use chrono::Utc as Timezone;
}

// TODO
#[cfg(feature = "embedded")]
fn create_local_path() -> std::path::PathBuf {
    use std::{fs, path::PathBuf};

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
