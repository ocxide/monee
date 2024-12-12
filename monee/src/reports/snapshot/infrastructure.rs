pub mod repository {
    use cream::context::FromContext;
    use monee_core::{Amount, DebtId, WalletId};

    use crate::{
        backoffice::{actors::domain::actor::Actor, currencies::domain::currency::Currency},
        reports::snapshot::domain::{
            self,
            snapshot::{Debt, Money, Snapshot, Wallet},
        },
        shared::{
            domain::context::DbContext,
            infrastructure::{
                database::{Connection, Entity},
                errors::InfrastructureError,
            },
        },
    };

    #[derive(FromContext)]
    #[context(DbContext)]
    pub struct SurrealRepository(Connection);

    #[derive(serde::Deserialize, Default)]
    struct SnapshotDto {
        wallets: Vec<SurrealWallet>,
        debts: Vec<Entity<DebtId, SurrealDebt>>,
        loans: Vec<Entity<DebtId, SurrealDebt>>,
    }

    #[derive(serde::Deserialize)]
    struct SurrealMoney {
        amount: Amount,
        #[serde(rename = "currency_id")]
        currency: Currency,
    }

    #[derive(serde::Deserialize)]
    struct SurrealWallet {
        #[serde(rename = "id")]
        data: Entity<WalletId, Wallet>,
        #[serde(flatten)]
        money: SurrealMoney,
    }

    #[derive(serde::Deserialize)]
    struct SurrealDebt {
        #[serde(flatten)]
        money: SurrealMoney,
        #[serde(flatten)]
        data: SurrealDebtData,
    }

    #[derive(serde::Deserialize)]
    struct SurrealDebtData {
        #[serde(rename = "actor_id")]
        actor: Actor,
    }

    impl From<SurrealMoney> for Money {
        fn from(value: SurrealMoney) -> Self {
            Money {
                amount: value.amount,
                currency: value.currency,
            }
        }
    }

    impl From<SurrealDebtData> for Debt {
        fn from(value: SurrealDebtData) -> Self {
            Debt { actor: value.actor }
        }
    }

    #[async_trait::async_trait]
    impl domain::repository::Repository for SurrealRepository {
        async fn read(&self) -> Result<Snapshot, InfrastructureError> {
            let mut response = self
                .0
                .query(
                    "SELECT * FROM snapshot FETCH 
wallets.currency_id, wallets.id,
debts.currency_id, debts.actor_id, 
loans.currency_id, loans.actor_id",
                )
                .await?
                .check()?;

            let snapshot: Option<SnapshotDto> = response.take(0)?;
            let snapshot = snapshot.unwrap_or_default();
            Ok(Snapshot {
                wallets: snapshot
                    .wallets
                    .into_iter()
                    .map(|w| (w.data.0, (w.data.1, w.money.into())))
                    .collect(),
                debts: snapshot
                    .debts
                    .into_iter()
                    .map(|d| (d.0, (d.1.data.into(), d.1.money.into())))
                    .collect(),
                loans: snapshot
                    .loans
                    .into_iter()
                    .map(|d| (d.0, (d.1.data.into(), d.1.money.into())))
                    .collect(),
            })
        }
    }
}

