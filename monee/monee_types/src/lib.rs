pub mod backoffice;
pub mod host;
pub mod reports;

pub mod shared {
    pub mod alias;
    pub mod date {
        pub type Datetime = chrono::DateTime<chrono::Utc>;
        pub use chrono::Utc as Timezone;
    }

    pub mod errors {
        #[derive(serde::Serialize)]
        pub enum UniqueSaveError {
            AlreadyExists,
        }
    }
}
