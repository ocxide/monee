pub mod local {
    pub(crate) mod fut_tracker {
        use std::sync::Mutex;

        use futures_channel::oneshot::Sender;
        use futures_util::future::FusedFuture;
        use futures_util::select;
        use leptos::task::spawn_local;

        #[derive(Default)]
        pub struct FutTracker {
            cancel_tx: Option<Sender<()>>,
        }

        pub struct FutTrackerMutex(Mutex<FutTracker>);

        impl FutTrackerMutex {
            pub fn spawn_local(&self, fut: impl FusedFuture<Output = ()> + 'static) {
                self.0.lock().unwrap().spawn_local(fut);
            }
        }

        impl FutTracker {
            pub fn cancel_current(&mut self) {
                if let Some(cancel_tx) = self.cancel_tx.take() {
                    let _ = cancel_tx.send(());
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
    }

    pub mod resource {
        use std::ops::Deref;

        use futures_util::FutureExt;
        use leptos::prelude::*;

        use super::fut_tracker::FutTracker;

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
                let (output_rx, output_tx) = signal(None);
                let effect = Effect::new(move || {
                    let mut tracker = FutTracker::default();
                    let fut = func();

                    tracker.spawn_local(
                        async move {
                            let result = fut.await;
                            output_tx.set(Some(result));
                        }
                        .fuse(),
                    );
                });

                Self {
                    effect,
                    output_rx,
                    output_tx,
                }
            }
        }
    }

    pub mod action {
        use futures_util::FutureExt;
        use leptos::prelude::*;
        use std::sync::Arc;

        use super::fut_tracker::FutTrackerMutex;

        pub struct LocalAction<I, O: 'static, Fut> {
            inner: Arc<Inner<I, Fut>>,
            output: (ReadSignal<Option<O>>, WriteSignal<Option<O>>),
            pending: (ReadSignal<bool>, WriteSignal<bool>),
        }

        struct Inner<I, Fut> {
            tracker: FutTrackerMutex,
            func: Box<dyn Fn(I) -> Fut + 'static + Send + Sync>,
        }

        impl<I, O: 'static, Fut> Clone for LocalAction<I, O, Fut> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                    output: self.output,
                    pending: self.pending,
                }
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
                        tracker: Default::default(),
                        func: Box::new(func),
                    }),
                    output: signal(None),
                    pending: signal(false),
                }
            }

            pub fn dispatch(&self, input: I) {
                let output_signal = self.output.1;
                let fut = (self.inner.func)(input);

                let pending = self.pending.1;
                pending.set(true);

                self.inner.tracker.spawn_local(
                    async move {
                        let output = fut.await;
                        output_signal.set(Some(output));
                        pending.set(false);
                    }
                    .fuse(),
                );
            }

            pub fn output(&self) -> ReadSignal<Option<O>> {
                self.output.0
            }

            pub fn pending(&self) -> ReadSignal<bool> {
                self.pending.0
            }
        }

        impl<I, T: 'static, E: 'static, Fut> LocalAction<I, Result<T, E>, Fut>
        where
            T: Send + Sync + 'static,
            E: Send + Sync + 'static,
        {
            pub fn error(&self) -> impl Fn() -> Option<E>
            where
                E: Clone,
            {
                let output = self.output.0;
                move || output.with(|state| state.as_ref().and_then(|r| r.as_ref().err().cloned()))
            }

            pub fn is_err(&self) -> bool {
                let output = self.output.0;
                output.with(|state| state.as_ref().map(|r| r.is_err()).unwrap_or(false))
            }
        }
    }
}
