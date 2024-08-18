pub mod domain {
    pub mod repository {
        use crate::{
            reports::snapshot::application::snapshot_deps_dto::SnapshotDepsDto,
            shared::infrastructure::errors::InfrastructureError,
        };

        #[async_trait::async_trait]
        pub trait Repository {
            async fn read_deps(&self) -> Result<SnapshotDepsDto, InfrastructureError>;
        }
    }

    pub mod snapshot {
        use std::collections::HashMap;

        use monee_core::{Amount, DebtId, WalletId};

        use crate::backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
            wallets::domain::wallet_name::WalletName,
        };

        pub struct Snapshot {
            pub wallets: HashMap<WalletId, (Wallet, Money)>,
            pub debts: HashMap<DebtId, (Debt, Money)>,
        }

        pub struct Money {
            pub amount: Amount,
            pub currency: Currency,
        }

        pub struct Debt {
            pub actor: Actor,
        }

        pub struct Wallet {
            pub name: WalletName,
            pub description: String,
        }
    }
}

pub mod application {
    pub mod snapshot_deps_dto {
        use crate::backoffice::wallets::domain::wallet_name::WalletName;
        use crate::shared::infrastructure::database::Entity;
        use monee_core::{ActorId, CurrencyId, WalletId};

        use crate::backoffice::{
            actors::domain::actor::Actor, currencies::domain::currency::Currency,
        };

        pub struct SnapshotDepsDto {
            pub actors: Vec<Entity<ActorId, Actor>>,
            pub wallets: Vec<Entity<WalletId, WalletDto>>,
            pub currencies: Vec<Entity<CurrencyId, Currency>>,
        }

        #[derive(serde::Deserialize)]
        pub struct WalletDto {
            pub currency_id: CurrencyId,
            pub name: WalletName,
            pub description: String,
        }
    }

    pub mod snapshot_report {
        use cream::context::ContextProvide;

        use crate::{
            backoffice::snapshot::application::snapshot_io::SnapshotIO,
            reports::snapshot::domain::{
                repository::Repository,
                snapshot::{Snapshot, Wallet},
            },
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        use super::snapshot_deps_dto::SnapshotDepsDto;

        #[derive(ContextProvide)]
        #[provider_context(AppContext)]
        pub struct SnapshotReport {
            repository: Box<dyn Repository>,
            snapshot_io: SnapshotIO,
        }

        impl SnapshotReport {
            pub async fn run(&self) -> Result<Snapshot, InfrastructureError> {
                let snapshot = self.snapshot_io.read_last().await?;
                let SnapshotDepsDto {
                    wallets,
                    actors,
                    currencies,
                } = self.repository.read_deps().await?;

                let wallets = snapshot.wallets.into_iter().map(|(id, snapshot_wallet)| {
                    let wallet = wallets.iter().find(|w| w.0 == id).expect("to find wallet");
                    let currency = currencies
                        .iter()
                        .find(|c| c.0 == wallet.1.currency_id)
                        .expect("to find currency");

                    (
                        Wallet {
                            name: wallet.1.name.clone(),
                            description: wallet.1.description.clone(),
                        },
                        snapshot_wallet.money.clone(),
                    )
                });

                todo!()
            }
        }
    }
}

pub mod infrastructure {
    pub mod repository {
        use cream::context::ContextProvide;
        use monee_core::{ActorId, CurrencyId, WalletId};

        use crate::{
            backoffice::{actors::domain::actor::Actor, currencies::domain::currency::Currency},
            reports::snapshot::{
                application::snapshot_deps_dto::{SnapshotDepsDto, WalletDto},
                domain,
            },
            shared::{
                domain::context::DbContext,
                infrastructure::{
                    database::{Connection, Entity},
                    errors::InfrastructureError,
                },
            },
        };

        #[derive(ContextProvide)]
        #[provider_context(DbContext)]
        pub struct SurrealRepository(Connection);

        #[async_trait::async_trait]
        impl domain::repository::Repository for SurrealRepository {
            async fn read_deps(&self) -> Result<SnapshotDepsDto, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT * FROM actor")
                    .query("SELECT * FROM wallet")
                    .query("SELECT * FROM currency")
                    .await?
                    .check()?;

                let actors: Vec<Entity<ActorId, Actor>> = response.take(0)?;
                let wallets: Vec<Entity<WalletId, WalletDto>> = response.take(1)?;
                let currencies: Vec<Entity<CurrencyId, Currency>> = response.take(2)?;

                Ok(SnapshotDepsDto {
                    actors,
                    wallets,
                    currencies,
                })
            }
        }
    }
}
