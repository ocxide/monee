pub mod signal {
    use leptos::prelude::With;

    pub trait AppGetErr<E> {
        fn error(&self) -> Option<E>;
    }

    pub trait AppWithErr<E> {
        fn is_err(&self) -> bool;
    }

    impl<G, T, E> AppGetErr<E> for G
    where
        G: With<Value = Result<T, E>>,
        E: Clone,
    {
        fn error(&self) -> Option<E> {
            self.with(|v| v.as_ref().err().cloned())
        }
    }
}

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
    }

    pub mod action {
        use futures_util::FutureExt;
        use leptos::prelude::*;
        use std::{ops::Deref, sync::Arc};

        use crate::leptos_util::signal::{AppGetErr, AppWithErr};

        use super::fut_tracker::FutTrackerMutex;

        struct Inner<I, Fut> {
            tracker: FutTrackerMutex,
            func: Box<dyn Fn(I) -> Fut + 'static + Send + Sync>,
        }

        pub struct LocalDispatcher<I, O: 'static, Fut> {
            inner: Arc<Inner<I, Fut>>,
            output: WriteSignal<Option<O>>,
            pending: WriteSignal<bool>,
        }

        impl<I, O: 'static, Fut> Clone for LocalDispatcher<I, O, Fut> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                    output: self.output,
                    pending: self.pending,
                }
            }
        }

        impl<I, O, Fut> LocalDispatcher<I, O, Fut>
        where
            Fut: std::future::Future<Output = O> + 'static,
            I: 'static,
            O: Send + Sync + 'static,
        {
            pub fn dispatch(&self, input: I) {
                let output_signal = self.output;
                let fut = (self.inner.func)(input);

                let pending = self.pending;
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
        }

        pub struct ActionOutput<O> {
            output: ReadSignal<Option<O>>,
            pending: ReadSignal<bool>,
        }

        impl<O> ActionOutput<O> {
            pub fn pending(&self) -> bool {
                self.pending.get()
            }
        }

        impl<O> Clone for ActionOutput<O> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<O> Copy for ActionOutput<O> {}
        impl<O> Deref for ActionOutput<O> {
            type Target = ReadSignal<Option<O>>;
            fn deref(&self) -> &Self::Target {
                &self.output
            }
        }

        pub fn local_action<I, O, Fut>(
            action_fn: impl Fn(I) -> Fut + 'static + Send + Sync,
        ) -> (ActionOutput<O>, LocalDispatcher<I, O, Fut>)
        where
            Fut: std::future::Future<Output = O> + 'static,
            I: 'static,
            O: Send + Sync + 'static,
        {
            let output = signal(None);
            let pending = signal(false);

            let dispatcher = LocalDispatcher {
                inner: Arc::new(Inner {
                    tracker: Default::default(),
                    func: Box::new(action_fn),
                }),
                output: output.1,
                pending: pending.1,
            };

            let action_output = ActionOutput {
                output: output.0,
                pending: pending.0,
            };

            (action_output, dispatcher)
        }

        impl<T, E> AppGetErr<E> for ActionOutput<Result<T, E>>
        where
            T: Send + Sync + 'static,
            E: Clone + Send + Sync + 'static,
        {
            fn error(&self) -> Option<E> {
                self.deref()
                    .with(|v| v.as_ref().and_then(|r| r.as_ref().err()).cloned())
            }
        }

        impl<T, E> AppWithErr<E> for ActionOutput<Result<T, E>>
        where
            T: Send + Sync + 'static,
            E: Send + Sync + 'static,
        {
            fn is_err(&self) -> bool {
                self.deref()
                    .with(|v| v.as_ref().is_some_and(|r| r.is_err()))
            }
        }
    }
}
