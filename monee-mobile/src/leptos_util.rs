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
}

