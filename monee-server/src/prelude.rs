pub struct Entity<K, T>(pub K, pub T);

impl<K, T> From<Entity<K, T>> for (K, T) {
    fn from(entity: Entity<K, T>) -> Self {
        (entity.0, entity.1)
    }
}

impl<K, T> From<(K, T)> for Entity<K, T> {
    fn from((key, value): (K, T)) -> Self {
        Entity(key, value)
    }
}

impl<K, T> serde::Serialize for Entity<K, T>
where
    K: serde::Serialize,
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct EntitySerializer<'a, K, T> {
            #[serde(rename = "id")]
            key: &'a K,
            #[serde(flatten)]
            value: &'a T,
        }

        EntitySerializer {
            key: &self.0,
            value: &self.1,
        }
        .serialize(serializer)
    }
}

impl<'de, K, T> serde::Deserialize<'de> for Entity<K, T>
where
    K: serde::Deserialize<'de>,
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct EntityDeserializer<K, T> {
            #[serde(rename = "id")]
            key: K,
            #[serde(flatten)]
            value: T,
        }

        let EntityDeserializer { key, value } = EntityDeserializer::deserialize(deserializer)?;
        Ok(Entity(key, value))
    }
}

