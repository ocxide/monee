pub use surrealdb::Result;

#[cfg(feature = "file")]
pub type Connection  = surrealdb::Surreal<surrealdb::engine::local::Db>;
#[cfg(feature = "remote")]
pub type Connection = surrealdb::Surreal<surrealdb::engine::remote::ws::Client>;

pub type Error = surrealdb::Error;

#[cfg(feature = "file")]
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

#[cfg(feature = "file")]
async fn create_connection() -> surrealdb::Result<(Connection, bool)> {
    use surrealdb::engine::local::Db;

    let path = crate::create_local_path().join(DB_DIR);
    let exists = tokio::fs::try_exists(&path).await.unwrap_or_else(|_| {
        println!("WARNING: Failed to check if db exists");
        false
    });

    let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
        "file://{}",
        path.display()
    ))
    .await?;

    (db, exists)
}

#[cfg(feature = "remote")]
async fn create_connection() -> surrealdb::Result<(Connection, bool)> {
    let db: Connection = surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>("0.0.0.0:6767").await?;
    Ok((db, false))
}

pub async fn connect() -> surrealdb::Result<Connection> {
    let (db, exists) = create_connection().await?;
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
