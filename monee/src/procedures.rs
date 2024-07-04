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
}

mod common {
    use super::{CreateProcedure, ProcedureType};

    pub struct ProcedureCreated {
        pub procedure_id: surrealdb::sql::Thing,
        pub snapshot: monee_core::Snapshot,
    }

    pub async fn create_procedure(
        connection: &crate::database::Connection,
        procedure: CreateProcedure,
        events: &[monee_core::Event],
        procedure_type: ProcedureType,
    ) -> Result<ProcedureCreated, crate::error::SnapshotReadError> {
        let crate::snapshot_io::SnapshotEntry { mut snapshot, .. } =
            crate::snapshot_io::read().await?;

        for event in events {
            snapshot.apply(event.clone())?;
        }

        let mut response = connection
            .query(surrealdb::sql::statements::BeginStatement)
            .query("LET $procedure = CREATE ONLY procedure SET description = $description, type = $type RETURN id")
            .bind(procedure)
            .bind(("type", procedure_type))
            .query("LET $events = INSERT INTO event $events_data")
            .bind(("events_data", events))
            .query("RELATE $procedure->generated->$events")
            .query(surrealdb::sql::statements::CommitStatement)
            .query("RETURN $procedure").await?.check()?;

        let procedure_id: Option<surrealdb::sql::Thing> =
            response.take((response.num_statements() - 1, "id"))?;

        Ok(ProcedureCreated {
            procedure_id: procedure_id.expect("to get procedure_id"),
            snapshot,
        })
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

        let response = common::create_procedure(
            connection,
            procedure,
            &events,
            ProcedureType::RegisterBalance,
        )
        .await?;

        crate::snapshot_io::write(response.snapshot).await?;
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

        let response = common::create_procedure(
            connection,
            procedure,
            &events,
            ProcedureType::RegisterInDebt,
        )
        .await?;

        connection
            .query("LET $actor = type::thing('actor', $actor_id)")
            .bind(("actor_id", plan.actor_id))
            .query(
                "RELATE $actor -> in_debt_on -> $procedure SET payment_promise = $payment_promise",
            )
            .bind(("procedure", response.procedure_id))
            .bind(("payment_promise", plan.payment_promise))
            .await?
            .check()?;

        crate::snapshot_io::write(response.snapshot).await?;
        Ok(())
    }
}
