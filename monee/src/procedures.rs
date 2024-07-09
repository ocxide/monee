#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateProcedure {
    pub description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureType {
    RegisterBalance,
    RegisterDebt,
    RegisterLoan,
    MoveValue,
    Buy,
}

pub mod list;

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

pub mod register_debt {
    use super::{common, CreateProcedure, ProcedureType};

    pub struct Plan {
        pub amount: monee_core::Amount,
        pub currency: monee_core::CurrencyId,
        pub actor_id: monee_core::actor::ActorId,
        pub payment_promise: Option<crate::date::Datetime>,
    }

    async fn run(
        connection: &crate::database::Connection,
        procedure: CreateProcedure,
        plan: Plan,
        procedure_type: ProcedureType,
        build_event: fn(monee_core::DebtEvent) -> monee_core::Event,
        relation: &str,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let debt_id = monee_core::DebtId::new();

        let events = [
            (build_event)(monee_core::DebtEvent::Incur {
                currency: plan.currency,
                debt_id,
            }),
            (build_event)(monee_core::DebtEvent::Accumulate {
                debt_id,
                amount: plan.amount,
            }),
        ];

        let entry = crate::snapshot_io::read().await?;

        common::create_procedure(connection, entry, procedure, &events, procedure_type, |q| {
            q.query("LET $actor = type::thing('actor', $actor_id)")
                .bind(("actor_id", plan.actor_id))
                .query(format!(
                    "RELATE $procedure->{relation}->$actor SET payment_promise = <option<datetime>>$payment_promise",
                ))
                .bind(("payment_promise", plan.payment_promise))
        })
        .await?;

        Ok(())
    }

    pub async fn run_debt(
        db: &crate::database::Connection,
        procedure: CreateProcedure,
        plan: Plan,
    ) -> Result<(), crate::error::SnapshotOptError> {
        run(
            db,
            procedure,
            plan,
            ProcedureType::RegisterDebt,
            |event| monee_core::Event::Debt(event),
            "debts",
        )
        .await
    }

    pub async fn run_loan(
        db: &crate::database::Connection,
        procedure: CreateProcedure,
        plan: Plan,
    ) -> Result<(), crate::error::SnapshotOptError> {
        run(
            db,
            procedure,
            plan,
            ProcedureType::RegisterLoan,
            |event| monee_core::Event::Loan(event),
            "loans",
        )
        .await
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

pub mod buy {
    use super::{common, ProcedureType};

    pub struct Plan {
        pub wallet_id: monee_core::WalletId,
        pub amount: monee_core::Amount,
        pub items: Vec<monee_core::item_tag::ItemTagId>,
        pub from_actors: Vec<monee_core::actor::ActorId>,
    }

    pub async fn run(
        db: &crate::database::Connection,
        procedure: super::CreateProcedure,
        plan: Plan,
    ) -> Result<(), crate::error::SnapshotOptError> {
        let entry = crate::snapshot_io::read().await?;

        let events = [monee_core::Event::Wallet(monee_core::WalletEvent::Deduct {
            wallet_id: plan.wallet_id,
            amount: plan.amount,
        })];

        common::create_procedure(db, entry, procedure, &events, ProcedureType::Buy, |q| {
            let item_tags = plan
                .items
                .iter()
                .map(|item| {
                    let id = surrealdb::sql::Id::String(item.to_string());
                    surrealdb::sql::Thing::from(("item_tag", id))
                })
                .collect::<Vec<_>>();

            let from_actors = plan
                .from_actors
                .iter()
                .map(|from_actor| {
                    let id = surrealdb::sql::Id::String(from_actor.to_string());
                    surrealdb::sql::Thing::from(("actor", id))
                })
                .collect::<Vec<_>>();

            q.query("LET $items_tags = $items_tags_values")
                .bind(("items_tags_values", item_tags))
                .query("RELATE $procedure->bought->$items_tags")
                .query("LET $from_actors = $from_actors_values")
                .bind(("from_actors_values", from_actors))
                .query("RELATE $procedure->bought->$from_actors")
        })
        .await
    }
}
