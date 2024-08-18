pub use surrealdb::Result;

#[cfg(feature = "embedded")]
pub type Engine = surrealdb::engine::local::Db;
#[cfg(feature = "remote")]
pub type Engine = surrealdb::engine::remote::ws::Client;

pub type Connection = surrealdb::Surreal<Engine>;

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
        .query("DEFINE TABLE wallet")
        .query("DEFINE FIELD name ON wallet TYPE option<string>")
        .query("DEFINE FIELD currency_id ON wallet TYPE record<currency>")
        .query("DEFINE INDEX wallet_name ON wallet_metadata FIELDS name UNIQUE")
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

pub(crate) mod entity_key {
    use monee_core::{ActorId, CurrencyId, DebtId, WalletId};
    use serde::{Deserialize, Serialize};

    use super::entity::de::SqlIdDe;

    pub struct EntityKey<K>(pub K);

    impl<'de, K: SqlIdDe> Deserialize<'de> for EntityKey<K> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let id = <K as SqlIdDe>::deserialize(deserializer)?;
            Ok(Self(id))
        }
    }

    // All ids should be lightweight, so its ok to impl Copy
    pub(in super::super) trait SqlIdSe: Copy {
        const TABLE: &'static str;

        fn into_id(self) -> surrealdb::sql::Id;
    }

    impl<K: SqlIdSe> Serialize for EntityKey<K> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let thing = surrealdb::sql::Thing {
                tb: K::TABLE.to_owned(),
                id: self.0.into_id(),
            };

            thing.serialize(serializer)
        }
    }

    impl SqlIdSe for DebtId {
        const TABLE: &'static str = "debt";
        fn into_id(self) -> surrealdb::sql::Id {
            surrealdb::sql::Id::String(self.to_string())
        }
    }

    impl SqlIdSe for WalletId {
        const TABLE: &'static str = "wallet";
        fn into_id(self) -> surrealdb::sql::Id {
            surrealdb::sql::Id::String(self.to_string())
        }
    }

    impl SqlIdSe for ActorId {
        const TABLE: &'static str = "actor";
        fn into_id(self) -> surrealdb::sql::Id {
            surrealdb::sql::Id::String(self.to_string())
        }
    }

    impl SqlIdSe for CurrencyId {
        const TABLE: &'static str = "currency";
        fn into_id(self) -> surrealdb::sql::Id {
            surrealdb::sql::Id::String(self.to_string())
        }
    }
}

pub(crate) mod entity {
    #[derive(Debug, Clone)]
    pub struct Entity<K, T>(pub K, pub T);

    impl<K, T> Entity<K, T> {
        pub fn into_inner(self) -> (K, T) {
            (self.0, self.1)
        }
    }

    impl<K, T> From<(K, T)> for Entity<K, T> {
        fn from(value: (K, T)) -> Self {
            Self(value.0, value.1)
        }
    }

    pub(in super::super) mod se {
        use crate::shared::infrastructure::database::entity_key::{EntityKey, SqlIdSe};

        use super::Entity;
        use serde::Serialize;

        #[derive(Serialize)]
        struct EntitySe<'a, K, T> {
            id: &'a K,
            #[serde(flatten)]
            value: &'a T,
        }

        impl<K, T> Serialize for Entity<K, T>
        where
            K: SqlIdSe,
            T: Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let id = EntityKey::<K>(self.0);
                EntitySe {
                    id: &id,
                    value: &self.1,
                }
                .serialize(serializer)
            }
        }
    }

    pub(in super::super) mod de {
        use super::Entity;
        use serde::Deserialize;

        pub trait SqlIdDe: Sized + serde::de::DeserializeOwned {
            fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Self, D::Error>;
        }

        impl<'de, K, T> Deserialize<'de> for Entity<K, T>
        where
            K: SqlIdDe,
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
        struct EntityBuilder<K: SqlIdDe, T> {
            #[serde(deserialize_with = "<ThindId<K>>::deserialize")]
            id: ThindId<K>,
            #[serde(flatten)]
            value: T,
        }

        #[derive(Deserialize)]
        struct ThindId<IK: SqlIdDe> {
            #[serde(deserialize_with = "<IK as SqlIdDe>::deserialize")]
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
                impl SqlIdDe for $name {
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
        impl_str_de!(monee_core::ActorId);
        impl_str_de!(monee_core::ItemTagId);
    }
}
