pub use surrealdb::Result;

#[cfg(feature = "embedded")]
pub type Engine = surrealdb::engine::local::Db;
#[cfg(feature = "remote")]
pub type Engine = surrealdb::engine::remote::ws::Client;
#[cfg(feature = "db_test")]
pub type Engine = surrealdb::engine::local::Db;

pub type Connection = surrealdb::Surreal<Engine>;

pub type Error = surrealdb::Error;

#[cfg(feature = "embedded")]
const DB_DIR: &str = "monee.db";

async fn init(connection: &Connection) -> Result<()> {
    init_backoffice(connection).await?;
    init_host(connection).await?;

    Ok(())
}

async fn init_backoffice(connection: &Connection) -> Result<()> {
    connection
        .query("DEFINE TABLE event")
        .query("DEFINE FIELD created_at ON event VALUE time::now()")
        .await?
        .check()?;

    connection
        .query("DEFINE TABLE wallet")
        .query("DEFINE FIELD name ON wallet TYPE option<string>")
        .query("DEFINE FIELD description ON wallet TYPE string")
        .query("DEFINE FIELD currency_id ON wallet TYPE record<currency>")
        .query("DEFINE INDEX wallet_name ON wallet FIELDS name UNIQUE")
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

    Ok(())
}

async fn init_host(connection: &Connection) -> Result<()> {
    connection
        .query("DEFINE TABLE client")
        .query("DEFINE FIELD name ON host TYPE option<string>")
        .await?
        .check()?;

    Ok(())
}

#[cfg(feature = "embedded")]
async fn create_connection(base_dir: std::path::PathBuf) -> surrealdb::Result<(Connection, bool)> {
    let path = base_dir.join(DB_DIR);
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

#[cfg(feature = "db_test")]
pub async fn create_connection() -> surrealdb::Result<(Connection, bool)> {
    let db = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(()).await?;
    Ok((db, false))
}

#[cfg(feature = "remote")]
async fn create_connection(url: String) -> surrealdb::Result<(Connection, bool)> {
    let db: Connection = surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(url).await?;
    Ok((db, false))
}

pub async fn connect(
    #[cfg(feature = "embedded")] base_dir: std::path::PathBuf,
    #[cfg(feature = "remote")] url: String,
) -> surrealdb::Result<Connection> {
    #[cfg(feature = "embedded")]
    let (db, exists) = create_connection(base_dir).await?;
    #[cfg(feature = "remote")]
    let (db, exists) = create_connection(url).await?;
    #[cfg(feature = "db_test")]
    let (db, exists) = create_connection().await?;

    db.use_ns("monee").use_db("monee").await?;

    if !exists {
        init(&db).await?;
    }

    Ok(db)
}

pub(crate) use entity::{Entity, EntityKey};

mod entity {
    use de::SqlIdDeserializator;
    use monee_types::apps::app_id::AppId;
    use se::SqlIdSerializator;
    use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};

    pub struct EntityKey<K>(pub K);
    pub struct Entity<K, T>(pub K, pub T);

    impl<K, T> From<(K, T)> for Entity<K, T> {
        fn from((key, value): (K, T)) -> Self {
            Entity(key, value)
        }
    }

    impl<K, T> From<Entity<K, T>> for (K, T) {
        fn from(Entity(key, value): Entity<K, T>) -> Self {
            (key, value)
        }
    }

    impl<K, T> Entity<K, T> {
        pub fn into_key(self) -> K {
            self.0
        }

        pub fn into_inner(self) -> (K, T) {
            self.into()
        }
    }

    pub(crate) mod de {
        use serde::{Deserialize, Deserializer};

        use super::{SqlId, StringId};

        pub trait SqlIdDeserializator<K> {
            fn deserialize_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<K, D::Error>;
        }

        impl<K: SqlId> SqlIdDeserializator<K> for StringId {
            fn deserialize_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<K, D::Error> {
                #[derive(serde::Deserialize)]
                struct SqlTable<I> {
                    id: I,
                }

                #[derive(serde::Deserialize)]
                struct StringSqlIdDeserialize<K> {
                    #[serde(rename = "String")]
                    field: K,
                }

                let val = SqlTable::<StringSqlIdDeserialize<K>>::deserialize(deserializer)?;
                Ok(val.id.field)
            }
        }
    }

    pub(crate) mod se {
        use std::fmt::Display;

        use super::{SqlId, StringId};

        pub trait SqlIdSerializator<K> {
            fn create_id(k: K) -> surrealdb::sql::Id;
        }

        impl<K: SqlId + Display> SqlIdSerializator<K> for StringId {
            fn create_id(k: K) -> surrealdb::sql::Id {
                surrealdb::sql::Id::String(k.to_string())
            }
        }
    }

    pub struct StringId;

    pub trait SqlId: Copy + DeserializeOwned {
        #[allow(private_bounds)]
        type Flavor: SqlIdDeserializator<Self> + SqlIdSerializator<Self>;

        const TABLE: &'static str;
    }

    impl<'de, K: SqlId> Deserialize<'de> for EntityKey<K> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let id = K::Flavor::deserialize_id(deserializer)?;
            Ok(EntityKey(id))
        }
    }

    impl<K: SqlId> Serialize for EntityKey<K> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let thing = surrealdb::sql::Thing {
                tb: K::TABLE.to_owned(),
                id: K::Flavor::create_id(self.0),
            };

            thing.serialize(serializer)
        }
    }

    impl<'de, K: SqlId, T: DeserializeOwned> Deserialize<'de> for Entity<K, T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(serde::Deserialize)]
            #[serde(bound = "T: DeserializeOwned")]
            struct EntityDe<K: SqlId, T> {
                id: EntityKey<K>,
                #[serde(flatten)]
                value: T,
            }
            let EntityDe { id: key, value } = EntityDe::deserialize(deserializer)?;
            Ok(Entity(key.0, value))
        }
    }

    impl<'de, K: SqlId, T: Serialize> Serialize for Entity<K, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            #[derive(serde::Serialize)]
            #[serde(bound = "T: Serialize")]
            struct EntitySer<'de, K: SqlId, T> {
                id: &'de EntityKey<K>,
                #[serde(flatten)]
                value: &'de T,
            }

            let key = EntityKey(self.0);
            EntitySer {
                id: &key,
                value: &self.1,
            }
            .serialize(serializer)
        }
    }

    impl SqlId for monee_core::WalletId {
        type Flavor = StringId;
        const TABLE: &'static str = "wallet";
    }

    impl SqlId for monee_core::CurrencyId {
        type Flavor = StringId;
        const TABLE: &'static str = "currency";
    }

    impl SqlId for monee_core::ActorId {
        type Flavor = StringId;
        const TABLE: &'static str = "actor";
    }

    impl SqlId for monee_core::ItemTagId {
        type Flavor = StringId;
        const TABLE: &'static str = "item_tag";
    }

    impl SqlId for monee_core::DebtId {
        type Flavor = StringId;
        const TABLE: &'static str = "debt";
    }

    impl SqlId for AppId {
        type Flavor = StringId;
        // TODO
        const TABLE: &'static str = "node";
    }

    impl SqlId for monee_core::EventId {
        type Flavor = StringId;
        const TABLE: &'static str = "event";
    }
}
