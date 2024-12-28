use components::host_status_bar::HostStatusBar;
use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route, Router, Routes},
    path,
};

mod components;
mod forms;
mod pages;

#[component]
fn AppLayout() -> impl IntoView {
    view! {
        <>
            <HostStatusBar />
            <main class="h-full gap-4 container mx-auto px-4 app-layout-main">
                <Outlet />
            </main>
        </>
    }
}

pub mod app_state {
    use codee::string::JsonSerdeCodec;
    use js_sys::Reflect;
    use leptos::prelude::*;
    use leptos_use::storage::use_session_storage;
    use serde_wasm_bindgen::from_value;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{prelude::Closure, JsValue};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
        fn listen(event: &str, callback: JsValue) -> wasm_bindgen::JsValue;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    pub enum HostStatus {
        Online,
        Offline,
    }

    pub fn use_host_status() -> Signal<Option<HostStatus>> {
        use_context::<Signal<Option<HostStatus>>>().expect("host status not provided")
    }

    pub fn setup() {
        let (host_status, set_host_status, _) =
            use_session_storage::<Option<HostStatus>, JsonSerdeCodec>("host_status_change");

        provide_context(host_status);

        let _ = listen(
            "host_status_change",
            Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
                let payload = Reflect::get(&event, &"payload".into()).unwrap();
                let status = from_value::<HostStatus>(payload).unwrap();
                set_host_status.set(Some(status));
            })
            .into_js_value(),
        );
    }
}

#[component]
pub fn App() -> impl IntoView {
    app_state::setup();

    use pages::event::{move_value::MoveValueForm, purchase::PurchaseForm, EventPageForm};

    view! {
        <Router>
                <Routes fallback=|| view! { <p>"not foun, da heck? " <a href="/home">"Go Back"</a></p> }>
                    <Route path=path!("/") view=pages::startup::StartUp />
                    <ParentRoute path=path!("/*") view=AppLayout>
                        <Route path=path!("/home") view=pages::home::Home />
                        <Route path=path!("/events/purchase") view=move || view! { <EventPageForm<PurchaseForm> /> } />
                        <Route path=path!("/events/move-value") view=move || view! { <EventPageForm<MoveValueForm> /> } />
                    </ParentRoute>
                </Routes>
        </Router>
    }
}
