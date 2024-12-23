use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

mod pages;
mod forms;
mod components;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="bg-slate-900 h-full w-screen grid text-white gap-4">
                <Routes fallback=|| view! { <p>"not foun, da heck? " <a href="/home">"Go Back"</a></p> }>
                    <Route path=path!("/home") view=pages::home::Home />
                    <Route path=path!("/") view=pages::startup::StartUp />
                    <Route path=path!("/events/purchase") view=pages::purchase::Purchase />
                </Routes>
            </main>
        </Router>
    }
}
