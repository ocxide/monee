use crate::create_local_path;
use surrealdb::engine::local::Db;
pub use surrealdb::Result;

pub type Connection = surrealdb::Surreal<Db>;
pub type Error = surrealdb::Error;

const DB_DIR: &str = "twon.db";

async fn init(connection: &Connection) -> Result<()> {
    connection
        .query("DEFINE TABLE event")
        .query("DEFINE FIELD created_at ON event VALUE time::now()")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE wallet_metadata")
        .query("DEFINE FIELD name ON wallet_metadata TYPE option<string>")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE currency")
        .query("DEFINE FIELD name ON currency TYPE string")
        .query("DEFINE FIELD symbol ON currency TYPE string")
        .query("DEFINE FIELD code ON currency TYPE string")
        .query("DEFINE INDEX currency_code ON currency FIELDS code UNIQUE")
        .await?
        .check()?;

    Ok(())
}

pub async fn connect() -> surrealdb::Result<Connection> {
    let path = create_local_path().join(DB_DIR);
    let exists = tokio::fs::try_exists(&path).await.unwrap_or_else(|_| {
        println!("WARNING: Failed to check if db exists");
        false
    });

    let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
        "file://{}",
        path.display()
    ))
    .await?;
    db.use_ns("twon").use_db("twon").await?;

    if !exists {
        init(&db).await?;
    }

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
