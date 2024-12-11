use std::collections::HashMap;

use leptos::*;

use crate::tauri_interop::bind_command;

bind_command!(get_stats() -> Snapshot, InternalError);

#[derive(serde::Serialize, serde::Deserialize)]
pub enum InternalError {
    Auth,
    Unknown,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    wallets: HashMap<String, f64>,
    debts: HashMap<String, f64>,
    loans: HashMap<String, f64>,
}

struct EventButton {
    name: &'static str,
    color: &'static str,
}

const EVENT_BUTTONS: &[EventButton] = &[
    EventButton {
        name: "Buy",
        color: "border-green-500",
    },
    EventButton {
        name: "Buy",
        color: "border-green-500",
    },
    EventButton {
        name: "Buy",
        color: "border-green-500",
    },
    EventButton {
        name: "Buy",
        color: "border-green-500",
    },
    EventButton {
        name: "Buy",
        color: "border-green-500",
    },
];

#[component]
pub fn App() -> impl IntoView {
    view! {
        <main class="bg-slate-900 h-full w-screen grid text-white place-content-center gap-4">
            <h1 class="text-4xl text-center">"Monee"</h1>

            <Stats />

            <ul class="flex flex-wrap gap-4 justify-center">
                {EVENT_BUTTONS.iter().map(|event| view! { <li><EventButton name=event.name color=event.color /></li> } ).collect::<Vec<_>>()}
            </ul>
        </main>
    }
}

#[component]
fn Stats() -> impl IntoView {
    let snapshot_rx = create_local_resource(|| {}, |_| get_stats());

    view! {
        <div>
        {move || snapshot_rx.with(|snapshot| match snapshot {
                Some(Ok(snapshot)) => view! { <pre>"!!!!"</pre> }.into_view(),
                Some(Err(e)) => view! { <pre>"err2r"</pre> }.into_view(),
                None => view! { <p>"Loading3..."</p> }.into_view(),
            })
        }
        </div>
    }
}

#[component]
fn EventButton(name: &'static str, color: &'static str) -> impl IntoView {
    view! {
        <button
            class=format!("p-8 text-xl rounded-full bg-slate-800 active:bg-slate-950 shadow-md shadow-slate-700 border-2 {color}")
        >
            {name}
        </button>
    }
}
