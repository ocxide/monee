mod custom_de {
    macro_rules! impl_deserialize {
        ($name:expr) => {
            pub use de::deserialize;

            pub(crate) mod de {
                use serde::{de::DeserializeOwned, Deserialize};

                pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
                where
                    T: DeserializeOwned,
                    D: serde::Deserializer<'de>,
                {
                    let thing = SqlThing::deserialize(deserializer)?;
                    Ok(thing.id.field)
                }

                #[derive(serde::Deserialize)]
                pub struct SqlThing<T> {
                    pub id: SqlInnerId<T>,
                }

                #[derive(serde::Deserialize)]
                pub struct SqlInnerId<T> {
                    #[serde(rename = $name)]
                    pub field: T,
                }
            }
        };
    }

    pub(crate) use impl_deserialize;
}

pub mod string {
    super::custom_de::impl_deserialize!("String");
}

pub mod string_vec {
    pub fn deserialize<'de, T: serde::Deserialize<'de>, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<T>, D::Error> {
        let items = <Vec<super::string::de::SqlThing<T>> as serde::Deserialize>::deserialize(deserializer)?
            .into_iter()
            .map(|thing| thing.id.field)
            .collect();

        Ok(items)
    }
}
