type Id = idn::IdN<4>;

mod id_utils {
    macro_rules! impl_id {
        ($name:ident, $inner_id:ty) => {
            impl $name {
                pub fn new() -> Self {
                    Self(<$inner_id>::new())
                }
            }

            impl Default for $name {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl std::str::FromStr for $name {
                type Err = <$inner_id as std::str::FromStr>::Err;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self(s.parse()?))
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
        };
    }

    pub(crate) use impl_id;
}

use id_utils::impl_id;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WalletId(Id);

crate::ids::impl_id!(WalletId, Id);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct DebtId(Id);

crate::ids::impl_id!(DebtId, Id);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ItemTagId(Id);

crate::ids::impl_id!(ItemTagId, Id);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CurrencyId(Id);

crate::ids::impl_id!(CurrencyId, Id);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ActorId(Id);

crate::ids::impl_id!(ActorId, Id);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct EventId(Id);

crate::ids::impl_id!(EventId, Id);
