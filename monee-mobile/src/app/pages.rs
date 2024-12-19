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
    use std::ops::Deref;

    use leptos::{ev::SubmitEvent, prelude::*, reactive::graph::Source, task::spawn_local};
    use leptos_router::hooks::use_navigate;
    use local_action::LocalAction;

    use crate::{bind_command, prelude::InternalError};

    bind_command!(set_host(host_dir: String) -> (), InternalError);

    mod resource_local {
        use std::ops::Deref;

        use futures_util::FutureExt;
        use leptos::prelude::*;

        use super::local_action::FutTracker;

        #[derive(Clone, Copy)]
        pub struct ResourceLocal<T: Send + Sync + 'static> {
            effect: Effect<LocalStorage>,
            output_rx: ReadSignal<Option<T>>,
            output_tx: WriteSignal<Option<T>>,
        }

        impl<T: Send + Sync + 'static> Deref for ResourceLocal<T> {
            type Target = ReadSignal<Option<T>>;
            fn deref(&self) -> &Self::Target {
                &self.output_rx
            }
        }

        impl<T: Send + Sync + 'static> ResourceLocal<T> {
            pub fn new<Fut>(func: impl Fn() -> Fut + 'static) -> Self
            where
                Fut: std::future::Future<Output = T> + 'static,
            {
                let effect = Effect::new(move || {
                    let mut tracker = FutTracker::default();
                    let fut = func();

                    tracker.spawn_local(
                        async move {
                            let result = fut.await;
                        }
                        .fuse(),
                    );
                });

                let (output_rx, output_tx) = signal(None);
                Self {
                    effect,
                    output_rx,
                    output_tx,
                }
            }
        }
    }

    pub mod local_action {
        use std::sync::{Arc, Mutex};

        use futures_channel::oneshot::Sender;
        use futures_util::future::FusedFuture;
        use futures_util::{select, FutureExt};
        use leptos::prelude::*;
        use leptos::task::spawn_local;

        pub struct LocalAction<I, O: 'static, Fut> {
            inner: Arc<Inner<I, O, Fut>>,
        }

        impl<I, O: 'static, Fut> Clone for LocalAction<I, O, Fut> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                }
            }
        }

        struct Inner<I, O: 'static, Fut> {
            func: Box<dyn Fn(I) -> Fut + Send + Sync>,
            fut_tracker: FutTrackerMutex,
            output: RwSignal<Option<O>>,
        }

        pub struct FutTrackerMutex(Mutex<FutTracker>);

        impl FutTrackerMutex {
            pub fn spawn_local(&self, fut: impl FusedFuture<Output = ()> + 'static) {
                self.0.lock().unwrap().spawn_local(fut);
            }
        }

        #[derive(Default)]
        pub struct FutTracker {
            cancel_tx: Option<Sender<()>>,
        }

        impl FutTracker {
            pub fn cancel_current(&mut self) {
                if let Some(cancel_tx) = self.cancel_tx.take() {
                    cancel_tx.send(()).unwrap();
                }
            }

            pub fn spawn_local(&mut self, fut: impl FusedFuture<Output = ()> + 'static) {
                self.cancel_current();

                let (tx, mut rx) = futures_channel::oneshot::channel();
                self.cancel_tx = Some(tx);

                // TODO: pin
                let mut fut = Box::pin(fut);

                spawn_local(async move {
                    select! {
                        _ = rx => {},
                        _ = fut => {}
                    }
                });
            }
        }

        impl Drop for FutTracker {
            fn drop(&mut self) {
                self.cancel_current();
            }
        }

        impl Default for FutTrackerMutex {
            fn default() -> Self {
                Self(Mutex::new(Default::default()))
            }
        }

        impl FutTrackerMutex {
            pub fn cancel_current(&self) {
                self.0.lock().unwrap().cancel_current();
            }
        }

        impl<I, O, Fut> LocalAction<I, O, Fut>
        where
            Fut: std::future::Future<Output = O> + 'static,
            I: 'static,
            O: Send + Sync + 'static,
        {
            pub fn new(func: impl Fn(I) -> Fut + 'static + Send + Sync) -> Self {
                Self {
                    inner: Arc::new(Inner {
                        func: Box::new(func),
                        fut_tracker: FutTrackerMutex::default(),
                        output: RwSignal::new(None),
                    }),
                }
            }

            pub fn dispatch(&self, input: I) {
                let output_signal = self.inner.output;
                let fut = (self.inner.func)(input);

                self.inner.fut_tracker.spawn_local(
                    async move {
                        let output = fut.await;
                        output_signal.set(Some(output));
                    }
                    .fuse(),
                );
            }

            pub fn output(&self) -> ReadSignal<Option<O>> {
                self.inner.output.read_only()
            }
        }
    }

    #[component]
    pub fn StartUp() -> impl IntoView {
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
            <div class="grid place-content-center">
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
            </div>
        }
    }
}
