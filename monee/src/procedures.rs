#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateProcedure {
    pub description: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureType {
    RegisterBalance,
    RegisterInDebt,
    MoveValue,
}

mod common {
    use super::{CreateProcedure, ProcedureType};

    pub async fn create_procedure(
        db: &crate::database::Connection,
        crate::snapshot_io::SnapshotEntry { mut snapshot, .. }: crate::snapshot_io::SnapshotEntry,
        procedure: CreateProcedure,
        events: &[monee_core::Event],
        procedure_type: ProcedureType,
        post_fn: impl Fn(
            surrealdb::method::Query<crate::database::Engine>,
        ) -> surrealdb::method::Query<crate::database::Engine>,
    ) -> Result<(), crate::error::SnapshotOptError> {
        for event in events {
            snapshot.apply(event.clone())?;
        }

        let query = db
            .query(surrealdb::sql::statements::BeginStatement)
            .query("LET $procedure = CREATE ONLY procedure SET description = $description, type = $type RETURN id")
            .bind(procedure)
            .bind(("type", procedure_type))
            .query("LET $events = INSERT INTO event $events_data")
            .bind(("events_data", events))
            .query("RELATE $procedure->generated->$events");

        (post_fn)(query)
            .query(surrealdb::sql::statements::CommitStatement)
            .await?
            .check()?;

        crate::snapshot_io::write(snapshot).await?;

        Ok(())
    }
}

pub mod register_balance {
    use super::{common, CreateProcedure, ProcedureType};

    pub struct Plan {
        pub wallet_id: monee_core::WalletId,
        pub amount: monee_core::Amount,
    }

    pub async fn run(
        connection: &crate::database::Connection,
        procedure: CreateProcedure,
        plan: Plan,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let events = [monee_core::Event::Wallet(
            monee_core::WalletEvent::Deposit {
                wallet_id: plan.wallet_id,
                amount: plan.amount,
            },
        )];

        let entry = crate::snapshot_io::read().await?;

        common::create_procedure(
            connection,
            entry,
            procedure,
            &events,
            ProcedureType::RegisterBalance,
            |q| q,
        )
        .await?;

        Ok(())
    }
}

pub mod register_in_debt {
    use super::{common, CreateProcedure, ProcedureType};

    pub struct Plan {
        pub amount: monee_core::Amount,
        pub currency: monee_core::CurrencyId,
        pub actor_id: monee_core::actor::ActorId,
        pub payment_promise: Option<crate::date::Datetime>,
    }

    pub async fn run(
        connection: &crate::database::Connection,
        procedure: CreateProcedure,
        plan: Plan,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let debt_id = monee_core::DebtId::new();

        let events = [
            monee_core::Event::InDebt(monee_core::DebtEvent::Incur {
                currency: plan.currency,
                debt_id,
            }),
            monee_core::Event::InDebt(monee_core::DebtEvent::Accumulate {
                debt_id,
                amount: plan.amount,
            }),
        ];

        let entry = crate::snapshot_io::read().await?;

        common::create_procedure(
            connection,
            entry,
            procedure,
            &events,
            ProcedureType::RegisterInDebt,
            |q| {
                q.query("LET $actor = type::thing('actor', $actor_id)")
                .bind(("actor_id", plan.actor_id))
                .query("RELATE $actor -> in_debt_on -> $procedure SET payment_promise = $payment_promise")
                .bind(("payment_promise", plan.payment_promise))
            },
        )
        .await?;

        Ok(())
    }
}

pub mod move_value {
    use super::{common, ProcedureType};

    pub struct Plan {
        pub from: monee_core::WalletId,
        pub to: monee_core::WalletId,
        pub amount: monee_core::Amount,
    }

    pub enum Error {
        UnequalCurrencies,
        Snapshot(crate::error::SnapshotOptError),
    }

    pub async fn run(
        connection: &crate::database::Connection,
        procedure: super::CreateProcedure,
        plan: Plan,
    ) -> Result<(), Error> {
        let entry = crate::snapshot_io::read()
            .await
            .map_err(|e| Error::Snapshot(e.into()))?;

        let wallet_not_found = || -> Error {
            Error::Snapshot(crate::error::SnapshotOptError::SnapshotApply(
                monee_core::Error::Wallet(monee_core::money_record::Error::NotFound),
            ))
        };

        let wallets = entry.snapshot.wallets.as_ref();
        let from = wallets.get(&plan.from).ok_or_else(wallet_not_found)?;
        let to = wallets.get(&plan.to).ok_or_else(wallet_not_found)?;

        if from.currency != to.currency {
            return Err(Error::UnequalCurrencies);
        }

        let events = [
            monee_core::Event::Wallet(monee_core::WalletEvent::Deduct {
                wallet_id: plan.from,
                amount: plan.amount,
            }),
            monee_core::Event::Wallet(monee_core::WalletEvent::Deposit {
                wallet_id: plan.to,
                amount: plan.amount,
            }),
        ];

        common::create_procedure(
            connection,
            entry,
            procedure,
            &events,
            ProcedureType::MoveValue,
            |q| q,
        )
        .await
        .map_err(Error::Snapshot)?;

        Ok(())
    }
}
