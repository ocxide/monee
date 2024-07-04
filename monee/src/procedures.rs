#[derive(serde::Serialize)]
pub struct CreateProcedure {
    pub description: Option<String>,
}

#[derive(serde::Serialize)]
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
            .query("CREATE procedure SET description = $description, type = $type RETURN id")
            .bind(procedure)
            .bind(("type", procedure_type))
            .await?
            .check()?;

        let procedure_id: surrealdb::sql::Thing = response
            .take::<Vec<_>>("id")?
            .into_iter()
            .next()
            .expect("to get procedure id");

        for event in events {
            let mut response = connection
                .query("CREATE event CONTENT $data RETURN id")
                .bind(("data", event))
                .await?
                .check()?;

            let event_id: surrealdb::sql::Thing = response
                .take::<Vec<_>>("id")?
                .into_iter()
                .next()
                .expect("to get event id");

            connection
                .query("RELATE $procedure->generated->$event")
                .bind(("procedure", procedure_id.clone()))
                .bind(("event", event_id))
                .await?
                .check()?;
        }

        Ok(ProcedureCreated {
            procedure_id,
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
