use std::collections::HashMap;

use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_no_args(cmd: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch, js_name = invoke)]
    async fn invoke_catch(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

macro_rules! bind_command {
    ($name:ident() -> $ret_ok:ty, $ret_err:ty) => {
        pub async fn $name() -> Result<$ret_ok, $ret_err> {
            tauri_invoke::<$ret_ok, $ret_err, ()>(stringify!($name), &()).await
        }
    };
}

async fn tauri_invoke<
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned,
    Args: serde::Serialize,
>(
    cmd: &str,
    args: &Args,
) -> Result<T, E> {
    let response = invoke_catch(cmd, serde_wasm_bindgen::to_value(args).unwrap()).await;
    match response {
        Ok(val) => Ok(serde_wasm_bindgen::from_value(val).unwrap()),
        Err(e) => Err(serde_wasm_bindgen::from_value(e).unwrap()),
    }
}

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
                Some(Err(e)) => view! { <pre>"errir"</pre> }.into_view(),
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
