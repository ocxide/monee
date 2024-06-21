#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum ActorType {
    Natural,
    Business,
    FinancialEntity,
}

pub mod actor_type_from_str {
    use std::str::FromStr;

    use super::ActorType;

    #[derive(Debug)]
    pub struct Error {}

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "invalid actor type, must be 'natural', 'business', or 'financial_entity'")
        }
    }

    impl std::error::Error for Error {}

    impl FromStr for ActorType {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "natural" | "n" => Ok(Self::Natural),
                "business" | "b" => Ok(Self::Business),
                "financial_entity" | "f" => Ok(Self::FinancialEntity),
                _ => Err(Error {}),
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Actor {
    pub name: String,
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub alias: Option<String>,
}

type Id = crate::tiny_id::TinyId<4>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ActorId(Id);

crate::id_utils::impl_id!(ActorId, Id);
