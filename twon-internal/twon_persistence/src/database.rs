use crate::create_local_path;
use surrealdb::engine::local::Db;
pub use surrealdb::Result;

pub type Connection = surrealdb::Surreal<Db>;
const DB_DIR: &str = "twon.db";

async fn init(connection: &Connection) -> Result<()> {
    connection
        .query("DEFINE TABLE event")
        .query("DEFINE FIELD created_at ON event VALUE time::now()")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE wallet_metadata")
        .query("DEFINE FIELD id ON wallet_metadata TYPE int")
        .query("DEFINE FIELD name ON wallet_metadata TYPE option<string>")
        .await?
        .check()?;

    Ok(())
}

async fn setup(connection: &Connection) -> Result<()> {
    // Skip initialization if db exists
    match tokio::fs::try_exists(create_local_path().join(DB_DIR)).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(_) => {
            println!("WARNING: Failed to check if db exists");
        }
    };

    let result = init(connection).await;
    if result.is_err() {
        println!("WARNING: Failed to initialize db");
    }

    result
}

pub async fn connect() -> surrealdb::Result<Connection> {
    let path = create_local_path().join(DB_DIR);
    let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
        "file://{}",
        path.display()
    ))
    .await?;
    db.use_ns("twon").use_db("twon").await?;

    setup(&db).await?;

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

