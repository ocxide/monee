use leptos::{ev::SubmitEvent, prelude::*};
use leptos_router::hooks::use_navigate;

use crate::{
    app_state::use_host_status, bind_command, leptos_util::local::action::LocalAction,
    prelude::InternalError,
};

bind_command!(set_host(host_dir: String) -> (), InternalError);
bind_command!(is_synced() -> bool, InternalError);

#[component]
pub fn StartUp() -> impl IntoView {
    let host_status = use_host_status();

    let navigate = use_navigate();
    let is_synced = LocalAction::new(move |_: ()| async move { is_synced().await });

    Effect::new({
        let navigate = navigate.clone();
        move || {
            if host_status.get().is_some() {
                navigate("/home", Default::default());
            }
        }
    });

    Effect::new({
        let is_synced = is_synced.clone();
        move || {
            is_synced.dispatch(());
        }
    });

    let output_view = {
        let is_synced = is_synced.clone();
        let output = is_synced.output();

        move || {
            output
                .get()
                .map(|result| match result {
                    Ok(false) => view! { <StartUpForm /> }.into_any(),
                    Ok(true) => view! { <p>"Synced"</p> }.into_any(),
                    Err(_) => {
                        let is_synced = is_synced.clone();
                        view! { <button on:click=move |_| is_synced.dispatch(())>"Try again"</button> }
                            .into_any()
                    }
                })
                .unwrap_or_else(|| view! { <p>"Loading..."</p> }.into_any())
        }
    };

    view! {
        <div class="grid place-content-center">
            <h1>"Monee Mobile"</h1>
            {output_view}
        </div>
    }
}

#[component]
fn StartUpForm() -> impl IntoView {
    let navigate = use_navigate();
    let (host_dir, set_host_dir) = signal(String::default());

    let set_host_binding = LocalAction::new(move |host_dir: String| {
        let navigate = navigate.clone();
        async move {
            set_host(host_dir).await?;
            navigate("/home", Default::default());

            Ok(()) as Result<(), InternalError>
        }
    });

    let on_submit = {
        let set_host_binding = set_host_binding.clone();
        move |e: SubmitEvent| {
            e.prevent_default();
            set_host_binding.dispatch(host_dir.get());
        }
    };

    let error_view = move || {
        set_host_binding
            .output()
            .with(|state| state.as_ref().map(Result::is_err))
            .map(|is_err| view! { <Show when=move || is_err> <p>"Error"</p> </Show> })
    };
    view! {
        <>
            <form on:submit=on_submit>
                <input
                    type="text"
                    class="bg-slate-800 px-2 py-1"
                    on:input:target=move |e| set_host_dir.set(e.target().value())
                />
                <button>"Submit"</button>
            </form>

            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {error_view}
            </Suspense>
        </>
    }
}
