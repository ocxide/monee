pub mod home {
    use crate::{prelude::*, tauri_interop::bind_command};
    use leptos::prelude::*;
    use monee_types::reports::snapshot::snapshot::Snapshot;

    bind_command!(get_stats() -> Snapshot, InternalError);

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
    pub fn Home() -> impl IntoView {
        view! {
            <main class="bg-slate-900 h-full w-screen grid text-white place-content-center gap-4">
                <h1 class="text-4xl text-center">"Monee"</h1>

                <LoadStats />

                <ul class="flex flex-wrap gap-4 justify-center">
                    {EVENT_BUTTONS.iter().map(|event| view! { <li><EventButton name=event.name color=event.color /></li> } ).collect::<Vec<_>>()}
                </ul>
            </main>
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
                <div class="grid gap-x-4 grid-cols-3 place-items-center">
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
    fn EventButton(name: &'static str, color: &'static str) -> impl IntoView {
        view! {
            <a
                href="/"
                class=format!("p-8 text-xl rounded-full bg-slate-800 active:bg-slate-950 shadow-md shadow-slate-700 border-2 {color}")
            >
                {name}
            </a>
        }
    }
}

pub mod startup {
    use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
    use leptos_router::hooks::use_navigate;

    use crate::{bind_command, prelude::InternalError};

    bind_command!(set_host(host_dir: String) -> (), InternalError);

    #[component]
    pub fn StartUp() -> impl IntoView {
        let navigate = use_navigate();

        let (host_dir, set_host_dir) = signal(String::default());
        let (loading, set_loading) = signal(false);
        let (err, set_err) = signal::<Option<InternalError>>(None);

        let on_submit = move |e: SubmitEvent| {
            e.prevent_default();

            spawn_local({
                let host_dir = host_dir.get_untracked();
                let navigate = navigate.clone();

                set_loading.set(true);

                async move {
                    let err = set_host(host_dir).await.err();
                    let is_err = err.is_some();

                    set_err.set(err);
                    set_loading.set(false);

                    if !is_err {
                        navigate("/home", Default::default());
                    }
                }
            });
        };

        view! {
            <div class="grid place-content-center">
                <form on:submit=on_submit>
                    <input
                        type="text" 
                        class="bg-slate-800 px-2 py-1" 
                        on:input:target=move |e| set_host_dir.set(e.target().value()) 
                    />
                    <button>"Submit"</button>
                </form>

                <Show when=move || err.get().is_some()>
                    <p>"An error occurred"</p>
                </Show>

                <Show when=move || loading.get()>
                    <p>"Loading..."</p>
                </Show>
            </div>
        }
    }
}
