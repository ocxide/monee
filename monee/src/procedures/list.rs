use std::{collections::HashMap, rc::Rc};

use monee_core::{currency::Currency, metadata::WalletMetadata, CurrencyId, WalletId};

use crate::Entity;

pub type CurrencyEntity = (CurrencyId, Rc<Currency>);
pub type WalletEntity = (WalletId, Rc<WalletMetadata>);

pub enum ProcedureDetail {
    RegisterBalance {
        wallet: WalletEntity,
        amount: monee_core::Amount,
        currency: Option<CurrencyEntity>,
    },
    RegisterDebt(RegisterDebt),
    RegisterLoan(RegisterDebt),
    MoveValue {
        from: WalletEntity,
        to: WalletEntity,
        amount: monee_core::Amount,
        currency: Option<CurrencyEntity>,
    },
    Buy {
        wallet: WalletEntity,
        items: Vec<String>,
        amount: monee_core::Amount,
        currency: Option<CurrencyEntity>,
    },
}

pub struct ProcedureData {
    pub created_at: crate::date::Datetime,
    pub detail: ProcedureDetail,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct Procedure {
    pub id: surrealdb::sql::Thing,
    #[serde(rename = "type")]
    pub procedure_type: super::ProcedureType,
    pub created_at: crate::date::Datetime,
}

pub struct RegisterDebt {
    pub debt_id: monee_core::DebtId,
    pub amount: monee_core::Amount,
    pub currency: Option<CurrencyEntity>,
    pub actor: (monee_core::actor::ActorId, Rc<monee_core::actor::Actor>),
    pub payment_promise: Option<crate::date::Datetime>,
}

async fn get_debt(
    db: &crate::database::Connection,
    relation: &'static str,
    procedure: &Procedure,
    currencies: &HashMap<CurrencyId, Rc<Currency>>,
    actors: &HashMap<monee_core::actor::ActorId, Rc<monee_core::actor::Actor>>,
) -> Result<RegisterDebt, crate::database::Error> {
    let mut response = db
        .query("SELECT * FROM $procedure->generated->event ORDER BY created_at")
        .bind(("procedure", procedure.id.clone()))
        .query(format!("SELECT * FROM ONLY $procedure->{relation} LIMIT 1"))
        .await?
        .check()?;

    #[derive(serde::Deserialize, Debug)]
    struct DebtRelation {
        #[serde(with = "crate::sql_id::string", rename = "out")]
        actor_id: monee_core::actor::ActorId,
        payment_promise: Option<crate::date::Datetime>,
    }

    let events: Vec<monee_core::DebtEvent> = response.take(0)?;
    let debt: Option<DebtRelation> = response.take(1)?;

    let detail = match (
        <[monee_core::DebtEvent; 2] as TryFrom<_>>::try_from(events),
        debt,
    ) {
        (
            Ok(
                [monee_core::DebtEvent::Incur { debt_id, currency }, monee_core::DebtEvent::Accumulate { debt_id: _, amount }],
            ),
            Some(debt),
        ) => {
            let currency = currencies.get(&currency).map(|c| (currency, Rc::clone(c)));
            let actor = actors
                .get(&debt.actor_id)
                .map(|a| (debt.actor_id, Rc::clone(a)))
                .expect("to get actor");

            RegisterDebt {
                debt_id,
                amount,
                currency,
                actor,
                payment_promise: debt.payment_promise,
            }
        }
        (events, debt) => panic!("failed, got: {events:?}, {debt:?}"),
    };

    Ok(detail)
}

async fn get_detail(
    db: &crate::database::Connection,
    entry: &crate::snapshot_io::SnapshotEntry,
    procedure: &Procedure,
    wallets: &HashMap<WalletId, Rc<WalletMetadata>>,
    currencies: &HashMap<CurrencyId, Rc<Currency>>,
    actors: &HashMap<monee_core::actor::ActorId, Rc<monee_core::actor::Actor>>,
) -> Result<ProcedureDetail, crate::database::Error> {
    let detail = match procedure.procedure_type {
        super::ProcedureType::RegisterBalance => {
            let event: Option<monee_core::WalletEvent> = db
                .query("SELECT * FROM ONLY $procedure->generated->event LIMIT 1")
                .bind(("procedure", procedure.id.clone()))
                .await?
                .check()?
                .take(0)?;

            match event {
                Some(monee_core::WalletEvent::Deposit { wallet_id, amount }) => {
                    let wallet_metadata = wallets.get(&wallet_id).expect("to get wallet");
                    let wallet = entry
                        .snapshot
                        .wallets
                        .as_ref()
                        .get(&wallet_id)
                        .expect("to get wallet");

                    let currency = currencies
                        .get(&wallet.currency)
                        .map(|c| (wallet.currency, Rc::clone(c)));

                    ProcedureDetail::RegisterBalance {
                        wallet: (wallet_id, Rc::clone(wallet_metadata)),
                        amount,
                        currency,
                    }
                }
                _ => panic!("failed, got: {event:?}"),
            }
        }

        super::ProcedureType::RegisterDebt => {
            let detail = get_debt(db, "debts", &procedure, &currencies, &actors).await?;
            ProcedureDetail::RegisterDebt(detail)
        }

        super::ProcedureType::RegisterLoan => {
            let detail = get_debt(db, "loans", &procedure, &currencies, &actors).await?;
            ProcedureDetail::RegisterLoan(detail)
        }

        super::ProcedureType::MoveValue => {
            let events: Vec<monee_core::WalletEvent> = db
                .query("SELECT * FROM $procedure->generated->event ORDER BY created_at")
                .bind(("procedure", procedure.id.clone()))
                .await?
                .check()?
                .take(0)?;

            match <[monee_core::WalletEvent; 2] as TryFrom<_>>::try_from(events) {
                Ok(
                    [monee_core::WalletEvent::Deduct {
                        wallet_id: from_id,
                        amount,
                    }, monee_core::WalletEvent::Deposit {
                        wallet_id: to_id, ..
                    }],
                ) => {
                    let from = wallets.get(&from_id).expect("to get from wallet");
                    let to = wallets.get(&to_id).expect("to get to wallet");

                    let from_wallet = entry
                        .snapshot
                        .wallets
                        .as_ref()
                        .get(&from_id)
                        .expect("to get from wallet");

                    let currency = currencies
                        .get(&from_wallet.currency)
                        .map(|c| (from_wallet.currency, Rc::clone(c)));

                    ProcedureDetail::MoveValue {
                        from: (from_id, Rc::clone(from)),
                        to: (to_id, Rc::clone(to)),
                        amount,
                        currency,
                    }
                }
                events => panic!("failed, got: {events:?}"),
            }
        }

        super::ProcedureType::Buy => {
            let mut response = db
                .query("SELECT * FROM ONLY $procedure->generated->event LIMIT 1")
                .bind(("procedure", procedure.id.clone()))
                .query("SELECT name FROM $procedure->bought->item_tag")
                .await?
                .check()?;

            let event: Option<monee_core::WalletEvent> = response.take(0)?;
            let items: Vec<String> = response.take((1, "name"))?;

            match event {
                Some(monee_core::WalletEvent::Deduct { wallet_id, amount }) => {
                    let wallet_metadata = wallets.get(&wallet_id).expect("to get wallet");
                    let wallet = entry
                        .snapshot
                        .wallets
                        .as_ref()
                        .get(&wallet_id)
                        .expect("to get wallet");

                    let currency = currencies
                        .get(&wallet.currency)
                        .map(|c| (wallet.currency, Rc::clone(c)));

                    ProcedureDetail::Buy {
                        wallet: (wallet_id, Rc::clone(wallet_metadata)),
                        items,
                        amount,
                        currency,
                    }
                }
                _ => panic!("failed, got: {event:?}"),
            }
        }
    };

    Ok(detail)
}

pub async fn run(
    db: &crate::database::Connection,
    since: Option<crate::date::Datetime>,
    until: Option<crate::date::Datetime>,
) -> Result<Vec<ProcedureData>, crate::error::SnapshotReadError> {
    let entry = crate::snapshot_io::read().await?;

    let until = until.unwrap_or_else(|| {
        let now = crate::date::Timezone::now();
        now.clone()
            .checked_add_days(chrono::Days::new(1))
            .unwrap_or(now)
    });

    let mut response = db
        .query("SELECT * FROM wallet_metadata")
        .query("SELECT * FROM actor")
        .query("SELECT * FROM currency")
        .query("SELECT * FROM procedure WHERE created_at >= <datetime>$since AND created_at <= <datetime>$until")
        .bind(("since", since.unwrap_or_else(|| crate::date::Datetime::UNIX_EPOCH)))
        .bind(("until", until))
        .await?.check()?
    ;

    let (wallets, actors, currencies, procedures): (
        Vec<Entity<WalletId, WalletMetadata>>,
        Vec<Entity<monee_core::actor::ActorId, monee_core::actor::Actor>>,
        Vec<Entity<CurrencyId, Currency>>,
        Vec<Procedure>,
    ) = (
        response.take(0)?,
        response.take(1)?,
        response.take(2)?,
        response.take(3)?,
    );

    let wallets: HashMap<WalletId, Rc<WalletMetadata>> =
        wallets.into_iter().map(|w| (w.0, Rc::new(w.1))).collect();

    let actors: HashMap<monee_core::actor::ActorId, Rc<monee_core::actor::Actor>> =
        actors.into_iter().map(|a| (a.0, Rc::new(a.1))).collect();

    let currencies: HashMap<CurrencyId, Rc<Currency>> = currencies
        .into_iter()
        .map(|c| (c.0, Rc::new(c.1)))
        .collect();

    let mut result = Vec::new();
    for procedure in procedures {
        let detail = get_detail(db, &entry, &procedure, &wallets, &currencies, &actors).await?;
        result.push(ProcedureData {
            detail,
            created_at: procedure.created_at,
        })
    }

    Ok(result)
}
