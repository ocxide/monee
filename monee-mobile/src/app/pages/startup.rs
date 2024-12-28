use leptos::{ev::SubmitEvent, prelude::*};
use leptos_router::hooks::use_navigate;

use crate::{
    app::components::pending::PendingPulse,
    app_state::use_host_status,
    bind_command,
    leptos_util::{local::action::local_action, signal::AppWithErr},
    prelude::InternalError,
};

bind_command!(set_host(host_dir: String) -> (), InternalError);
bind_command!(is_synced() -> bool, InternalError);

#[component]
pub fn StartUp() -> impl IntoView {
    let host_status = use_host_status();

    let navigate = use_navigate();
    let (is_synced, dispatch) = local_action(move |_: ()| async move { is_synced().await });

    Effect::new({
        let navigate = navigate.clone();
        move || {
            if host_status.get().is_some() {
                navigate("/home", Default::default());
            }
        }
    });

    Effect::new({
        let dispatch = dispatch.clone();
        move || {
            dispatch.dispatch(());
        }
    });

    let output_view = {
        let dispatch = dispatch.clone();
        move || {
            let err = is_synced.is_err();

            if err {
                let dispatch = dispatch.clone();
                let v =
                    view! { <button on:click=move |_| dispatch.dispatch(())>"Try again"</button> };
                Some(v)
            } else {
                None
            }
        }
    };

    view! {
        <div class="grid place-content-center auto-rows-auto h-full gap-16">
            <h1 class="text-5xl text-center">"Monee"</h1>

            <Show when=move || is_synced.pending()>
                <div class="grid place-content-center">
                    <PendingPulse class="w-24" />
                </div>
            </Show>

            {output_view}


            <Show when=move || is_synced.with(|state| matches!(state, Some(Ok(false))))>
                <StartUpForm />
            </Show>
        </div>
    }
}

#[component]
fn StartUpForm() -> impl IntoView {
    let navigate = use_navigate();
    let (host_dir, set_host_dir) = signal(String::default());

    let (set_host_binding, dispatch) = local_action(move |host_dir: String| {
        let navigate = navigate.clone();
        async move {
            set_host(host_dir).await?;
            navigate("/home", Default::default());

            Ok(()) as Result<(), InternalError>
        }
    });

    let on_submit = move |e: SubmitEvent| {
        e.prevent_default();
        dispatch.dispatch(host_dir.get());
    };

    view! {
        <div class="grid gap-2">
            <form on:submit=on_submit class="flex gap-2">
                <input
                    type="text"
                    class="bg-slate-800 px-2 py-1"
                    on:input:target=move |e| set_host_dir.set(e.target().value())
                />

                <button class="bg-blue-500 px-2 py-1">"Submit"</button>
            </form>

            <Show when=move || set_host_binding.is_err()>
                <p class="text-red-600">"OH NO, there was an error wtf"</p>
            </Show>

            <Show when=move || set_host_binding.pending()>
                <p class="text-slate-300">"Loading..."</p>
            </Show>
        </div>
    }
}
