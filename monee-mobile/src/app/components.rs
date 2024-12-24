pub mod dialog_form {
    use leptos::{html::Dialog, prelude::*};
    use leptos_use::on_click_outside;

    #[component]
    fn DialogForm(
        children: Children,
        try_close: ReadSignal<bool>,
        on_close: impl Fn() + Send + Sync + 'static + Copy,
    ) -> impl IntoView {
        let dialog = NodeRef::<Dialog>::new();

        Effect::new(move |_| {
            if try_close.get() {
                dialog.get().unwrap().close();
                on_close();
            }
        });

        let _ = on_click_outside(dialog, move |_| {
            dialog.get().unwrap().close();
            on_close();
        });

        view! {
            <dialog open="true" node_ref=dialog class="text-white fixed inset-0 p-4 bg-gray-800 rounded-lg overflow-hidden backdrop:bg-white/50 backdrop:backdrop-blur-md">
                {children()}
            </dialog>
        }
    }

    pub trait EntityForm {
        type Id: 'static + Copy + Send + Sync;
        fn create_view(
            on_save: impl Fn(Self::Id) + Send + Sync + 'static + Copy,
        ) -> impl IntoView + 'static;
    }

    pub fn create_dialog<F: EntityForm>(
        on_save: impl Fn(F::Id) + Send + Sync + 'static + Copy,
    ) -> (impl Fn() + Send + Sync, impl IntoView) {
        let (openned, set_open) = signal(false);
        let (try_close, set_try_close) = signal(false);

        let id_store = StoredValue::new(None as Option<F::Id>);

        let on_form_save = move |id: F::Id| {
            set_try_close.set(true);
            id_store.set_value(Some(id));
        };
        let on_dialog_closed = move || {
            set_open.set(false);
            set_try_close.set(false);

            if let Some(id) = id_store.get_value() {
                on_save(id);
            }
        };

        let dialog = view! {
            <Show when=move || openned.get()>
                <DialogForm on_close=on_dialog_closed try_close=try_close>
                    {F::create_view(on_form_save)}
                </DialogForm>
            </Show>
        };

        let open = move || {
            set_open.set(true);
        };

        (open, dialog)
    }
}
