#[derive(serde::Deserialize, serde::Serialize)]
pub enum ActorType {
    Natural,
    Business,
    FinancialEntity,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Actor {
    pub name: String,
    #[serde(rename = "type")]
    pub actor_type: ActorType,
}

type Id = crate::tiny_id::TinyId<4>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ActorId(Id);

crate::id_utils::impl_id!(ActorId, Id);
