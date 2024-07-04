pub use surrealdb::Result;

#[cfg(feature = "embedded")]
pub type Connection = surrealdb::Surreal<surrealdb::engine::local::Db>;
#[cfg(feature = "remote")]
pub type Connection = surrealdb::Surreal<surrealdb::engine::remote::ws::Client>;

pub type Error = surrealdb::Error;

#[cfg(feature = "embedded")]
const DB_DIR: &str = "monee.db";

async fn init(connection: &Connection) -> Result<()> {
    connection
        .query("DEFINE TABLE event")
        .query("DEFINE FIELD created_at ON event VALUE time::now()")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE wallet_metadata")
        .query("DEFINE FIELD name ON wallet_metadata TYPE option<string>")
        .query("DEFINE INDEX wallet_metadata_name ON wallet_metadata FIELDS name UNIQUE")
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

    connection
        .query("DEFINE TABLE procedure")
        .query("DEFINE FIELD created_at ON procedure VALUE time::now()")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE actor")
        .query("DEFINE FIELD name ON actor TYPE string")
        .query("DEFINE FIELD type ON actor TYPE string")
        .query("DEFINE FIELD alias ON actor TYPE option<string>")
        .query("DEFINE INDEX actor_alias ON actor FIELDS alias UNIQUE")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE item_tag")
        .query("DEFINE FIELD name ON item_tag TYPE string")
        .query("DEFINE INDEX item_tag_name ON item_tag FIELDS name UNIQUE")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE contains")
        .query("DEFINE FIELD in ON contains TYPE record<item_tag>")
        .query("DEFINE FIELD out ON contains TYPE record<item_tag>")
        .query("DEFINE INDEX contains_item_tag ON contains FIELDS in, out UNIQUE")
        .await?
        .check()?;

    Ok(())
}

#[cfg(feature = "embedded")]
async fn create_connection() -> surrealdb::Result<(Connection, bool)> {
    let path = crate::create_local_path().join(DB_DIR);
    // For now, just run definition queries
    /* let exists = tokio::fs::try_exists(&path).await.unwrap_or_else(|_| {
        println!("WARNING: Failed to check if db exists");
        false
    }); */
    let exists = false;

    let db = surrealdb::Surreal::new::<surrealdb::engine::local::File>(format!(
        "file://{}",
        path.display()
    ))
    .await?;

    Ok((db, exists))
}

#[cfg(feature = "remote")]
async fn create_connection() -> surrealdb::Result<(Connection, bool)> {
    let db: Connection =
        surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>("0.0.0.0:6767").await?;
    Ok((db, false))
}

pub async fn connect() -> surrealdb::Result<Connection> {
    let (db, exists) = create_connection().await?;
    db.use_ns("monee").use_db("monee").await?;

    if !exists {
        init(&db).await?;
    }

    Ok(db)
}

pub use entity::Entity;

pub(crate) mod entity {
    use serde::Deserialize;

    #[derive(Debug, Clone)]
    pub struct Entity<K, T>(pub K, pub T);

    impl<K, T> Entity<K, T> {
        pub fn into_inner(self) -> (K, T) {
            (self.0, self.1)
        }
    }

    trait SqlId: Sized + serde::de::DeserializeOwned {
        fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D)
            -> Result<Self, D::Error>;
    }

    impl<'de, K, T> Deserialize<'de> for Entity<K, T>
    where
        K: SqlId,
        T: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let builder = EntityBuilder::<K, T>::deserialize(deserializer)?;
            Ok(Self(builder.id.id, builder.value))
        }
    }

    #[derive(Deserialize)]
    struct EntityBuilder<K: SqlId, T> {
        #[serde(deserialize_with = "<ThindId<K>>::deserialize")]
        id: ThindId<K>,
        #[serde(flatten)]
        value: T,
    }

    #[derive(Deserialize)]
    struct ThindId<IK: SqlId> {
        #[serde(deserialize_with = "<IK as SqlId>::deserialize")]
        id: IK,
    }

    mod sql_inner_id {
        #[derive(serde::Deserialize)]
        pub struct SqlStringId<K> {
            #[serde(rename = "String")]
            pub field: K,
        }
    }

    macro_rules! impl_str_de {
        ($name:path) => {
            impl SqlId for $name {
                fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Self, D::Error> {
                    let id = sql_inner_id::SqlStringId::<Self>::deserialize(deserializer)?;
                    Ok(id.field)
                }
            }
        };
    }

    impl_str_de!(monee_core::DebtId);
    impl_str_de!(monee_core::WalletId);
    impl_str_de!(monee_core::CurrencyId);
    impl_str_de!(monee_core::actor::ActorId);
    impl_str_de!(monee_core::item_tag::ItemTagId);
}
