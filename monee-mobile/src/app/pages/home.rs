use crate::{prelude::*, tauri_interop::bind_command};
use leptos::prelude::*;
use monee_types::reports::snapshot::snapshot::Snapshot;

bind_command!(get_stats() -> Snapshot, InternalError);

struct EventButton {
    name: &'static str,
    color: &'static str,
    href: &'static str,
}

const EVENT_BUTTONS: &[EventButton] = &[
    EventButton {
        name: "Purchase",
        color: "border-green-500",
        href: "/events/purchase",
    },
    EventButton {
        name: "Move value",
        color: "border-blue-500",
        href: "/events/move-value",
    },
];

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center h-full gap-20">
            <h1 class="text-5xl text-center">"Monee"</h1>

            <LoadStats />

            <ul class="flex flex-wrap gap-4 justify-center">
                {EVENT_BUTTONS.iter().map(|event| view! { <li><EventButton name=event.name color=event.color href=event.href /></li> } ).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

#[component]
fn ListView(children: Vec<impl IntoView>) -> impl IntoView {
    use leptos::either::Either;
    if children.is_empty() {
        Either::Right(view! { <p>"None"</p> })
    } else {
        Either::Left(view! {
            <ul>
                {children}
            </ul>
        })
    }
}

#[component]
fn LoadStats() -> impl IntoView {
    let snapshot_rx = LocalResource::new(get_stats);

    let stats = |snapshot: &Snapshot| {
        let wallets = snapshot
    .wallets
    .iter()
    .map(|(_, (wallet, money))| view! { <li>{format!("{}: {} {}{}", wallet.name, money.currency.code, money.currency.symbol, money.amount)}</li> })
    .collect_view();

        let debts = snapshot
    .debts
    .iter()
    .map(|(_, (debt, money))| view! { <li>{format!("{}: {} {}{}", debt.actor.name, money.currency.code, money.currency.symbol, money.amount)}</li> })
    .collect_view();

        let loan = snapshot
    .loans
    .iter()
    .map(|(_, (loan, money))| view! { <li>{format!("{}: {} {}{}", loan.actor.name, money.currency.code, money.currency.symbol, money.amount)}</li> })
    .collect_view();

        view! {
            <div class="flex justify-center">
                <div class="flex gap-6 justify-between flex-wrap max-w-md">
                    <div>
                        <p class="text-xl">"Wallets"</p>
                        <ListView children=wallets />
                    </div>
                    <div>
                        <p class="text-xl">"Debts"</p>
                        <ListView children=debts />
                    </div>
                    <div>
                        <p class="text-xl">"Loans"</p>
                        <ListView children=loan />
                    </div>
                </div>
            </div>
        }
    };

    let stats_load = move || {
        snapshot_rx.with(|state| {
            state.as_ref().map(|result| match result.as_ref() {
                Ok(snapshot) => stats(snapshot).into_any(),
                Err(_) => view! { <p>"Error"</p> }.into_any(),
            })
        })
    };

    view! {
        <div>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {stats_load}
            </Suspense>
        </div>
    }
}

#[component]
fn EventButton(name: &'static str, color: &'static str, href: &'static str) -> impl IntoView {
    view! {
        <a
            href=href
            class=format!("inline-block p-8 text-xl rounded-full bg-slate-800 active:bg-slate-950 shadow-md shadow-slate-700 border-2 {color}")
        >
            {name}
        </a>
    }
}
