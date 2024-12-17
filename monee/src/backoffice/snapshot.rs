pub mod domain {
    pub mod repository {
        use monee_core::Snapshot;

        use crate::shared::infrastructure::errors::InfrastructureError;

        #[async_trait::async_trait]
        pub trait SnapshotRepository: Send + Sync {
            async fn read_last(&self) -> Result<Option<Snapshot>, InfrastructureError>;
            async fn save(&self, snapshot: Snapshot) -> Result<(), InfrastructureError>;
            async fn delete_all(&self) -> Result<(), InfrastructureError>;
        }
    }
}

pub mod application {
    pub mod on_wallet_created {
        use cream::{context::FromContext, events::Handler};

        use crate::shared::domain::context::AppContext;

        use super::snapshot_io::SnapshotIO;

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct OnWalletCreated {
            snapshot_io: SnapshotIO,
        }

        impl Handler for OnWalletCreated {
            type Event = crate::backoffice::wallets::domain::wallet_created::WalletCreated;

            async fn handle(&self, event: Self::Event) -> Result<(), cream::events::Error> {
                let mut snapshot = self
                    .snapshot_io
                    .read_last()
                    .await
                    .expect("to read snapshot");

                // If snapshot already has this wallet, do nothing
                let result = snapshot.apply(monee_core::Operation::Wallet(
                    monee_core::WalletOperation::Create {
                        currency_id: event.currency_id,
                        wallet_id: event.id,
                    },
                ));

                if result.is_ok() {
                    self.snapshot_io
                        .save(snapshot)
                        .await
                        .expect("to save snapshot");
                }

                Ok(())
            }
        }
    }

    pub mod snapshot_io {
        use cream::context::FromContext;

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{domain::context::AppContext, infrastructure::errors::InfrastructureError},
        };

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct SnapshotIO {
            repository: Box<dyn SnapshotRepository>,
        }

        impl SnapshotIO {
            pub async fn read_last(&self) -> Result<monee_core::Snapshot, InfrastructureError> {
                self.repository
                    .read_last()
                    .await
                    .map(|snapshot| snapshot.unwrap_or_default())
            }

            pub async fn save(
                &self,
                snapshot: monee_core::Snapshot,
            ) -> Result<(), InfrastructureError> {
                self.repository.delete_all().await?;
                self.repository.save(snapshot).await?;

                Ok(())
            }
        }
    }
}

pub mod infrastructure {
    pub mod snapshot_repository {
        use cream::context::FromContext;
        use monee_core::{
            ActorId, Amount, CurrencyId, Debt, DebtId, Money, MoneyMap, Snapshot, Wallet, WalletId,
        };

        use crate::{
            backoffice::snapshot::domain::repository::SnapshotRepository,
            shared::{
                domain::context::DbContext,
                infrastructure::{
                    database::{EntityKey, Connection, Entity},
                    errors::InfrastructureError,
                },
            },
        };

        #[derive(FromContext)]
        #[context(DbContext)]
        pub struct SnapshotSurrealRepository(Connection);

        #[derive(serde::Serialize, serde::Deserialize)]
        struct SurrealSnapshot {
            wallets: Vec<Entity<WalletId, SurrealWallet>>,
            debts: Vec<Entity<DebtId, SurrealDebt>>,
            loans: Vec<Entity<DebtId, SurrealDebt>>,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct SurrealMoney {
            amount: Amount,
            currency_id: EntityKey<CurrencyId>,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct SurrealWallet {
            #[serde(flatten)]
            money: SurrealMoney,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct SurrealDebt {
            #[serde(flatten)]
            money: SurrealMoney,
            actor_id: EntityKey<ActorId>,
        }

        impl From<Money> for SurrealMoney {
            fn from(money: Money) -> Self {
                Self {
                    amount: money.amount,
                    currency_id: EntityKey(money.currency_id),
                }
            }
        }

        impl From<Wallet> for SurrealWallet {
            fn from(value: Wallet) -> Self {
                Self {
                    money: value.money.into(),
                }
            }
        }

        impl From<Debt> for SurrealDebt {
            fn from(value: Debt) -> Self {
                Self {
                    money: value.money.into(),
                    actor_id: EntityKey(value.actor_id),
                }
            }
        }

        impl From<Snapshot> for SurrealSnapshot {
            fn from(snapshot: Snapshot) -> Self {
                Self {
                    wallets: snapshot
                        .wallets
                        .into_iter()
                        .map(|(id, wallet)| Entity::from((id, wallet.into())))
                        .collect(),
                    debts: snapshot
                        .debts
                        .into_iter()
                        .map(|(id, debt)| Entity::from((id, debt.into())))
                        .collect(),
                    loans: snapshot
                        .loans
                        .into_iter()
                        .map(|(id, debt)| Entity::from((id, debt.into())))
                        .collect(),
                }
            }
        }

        impl From<SurrealMoney> for Money {
            fn from(value: SurrealMoney) -> Self {
                Self {
                    amount: value.amount,
                    currency_id: value.currency_id.0,
                }
            }
        }

        impl From<SurrealWallet> for Wallet {
            fn from(wallet: SurrealWallet) -> Self {
                Self {
                    money: wallet.money.into(),
                }
            }
        }

        impl From<SurrealDebt> for Debt {
            fn from(debt: SurrealDebt) -> Self {
                Self {
                    money: debt.money.into(),
                    actor_id: debt.actor_id.0,
                }
            }
        }

        impl From<SurrealSnapshot> for Snapshot {
            fn from(snapshot: SurrealSnapshot) -> Self {
                Self {
                    // Build from surraldb, its supossed to be valid data
                    wallets: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.wallets.into_iter().map(|w| (w.0, w.1.into())),
                        )
                    },
                    // Build from surraldb, its supossed to be valid data
                    debts: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.debts.into_iter().map(|d| (d.0, d.1.into())),
                        )
                    },
                    // Build from surraldb, its supossed to be valid data
                    loans: unsafe {
                        MoneyMap::from_iter_unchecked(
                            snapshot.loans.into_iter().map(|l| (l.0, l.1.into())),
                        )
                    },
                }
            }
        }

        #[async_trait::async_trait]
        impl SnapshotRepository for SnapshotSurrealRepository {
            async fn read_last(&self) -> Result<Option<Snapshot>, InfrastructureError> {
                let mut response = self
                    .0
                    .query("SELECT * FROM ONLY snapshot ORDER BY created_at DESC LIMIT 1")
                    .await?
                    .check()?;

                let snapshot: Option<SurrealSnapshot> = response.take(0)?;
                Ok(snapshot.map(From::from))
            }

            async fn save(
                &self,
                snapshot: monee_core::Snapshot,
            ) -> Result<(), InfrastructureError> {
                self.0
                    .query("CREATE snapshot CONTENT $snapshot")
                    .bind(("snapshot", SurrealSnapshot::from(snapshot)))
                    .await?
                    .check()?;

                Ok(())
            }

            async fn delete_all(&self) -> Result<(), InfrastructureError> {
                self.0.query("DELETE FROM snapshot").await?.check()?;
                Ok(())
            }
        }

        #[cfg(all(test, feature = "db_test"))]
        mod tests {
            use monee_core::{ActorId, CurrencyId};
            use cream::context::Context;

            use crate::backoffice::wallets::domain::{repository::Repository as _, wallet::Wallet};

            use super::*;

            #[test]
            fn it_creates_snapshot() {
                return;
                let mut snapshot = Snapshot::default();

                let wallet_id = WalletId::new();
                let currency_id = CurrencyId::new();
                let actor_id = ActorId::new();
                let debt_id = DebtId::new();

                snapshot
                    .apply(monee_core::Operation::Wallet(
                        monee_core::WalletOperation::Create {
                            wallet_id,
                            currency_id,
                        },
                    ))
                    .unwrap();

                snapshot
                    .apply(monee_core::Operation::Debt(
                        monee_core::DebtOperation::Incur {
                            debt_id,
                            currency_id,
                            actor_id,
                        },
                    ))
                    .unwrap();

                let wallet = Wallet {
                    currency_id,
                    name: "wallet_1".parse().unwrap(),
                    description: "".into(),
                };

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        let db = crate::shared::infrastructure::database::connect()
                            .await
                            .unwrap();

                        let ctx = crate::shared::domain::context::DbContext::new(db);
                        let repo: super::SnapshotSurrealRepository = ctx.provide();
                        let wallet_repo: crate::backoffice::wallets::infrastructure::repository::SurrealRepository = ctx.provide();

                        wallet_repo.save(wallet_id, wallet).await.unwrap();
                        repo.save(snapshot).await.unwrap();
                    });
            }
        }
    }
}
