mod custom_de {
    macro_rules! impl_deserialize {
        ($name:expr) => {
            pub use de::deserialize;

            mod de {
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
                struct SqlThing<T> {
                    pub id: SqlInnerId<T>,
                }

                #[derive(serde::Deserialize)]
                struct SqlInnerId<T> {
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
