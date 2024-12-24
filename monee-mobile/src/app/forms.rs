pub mod create_actor {
    use leptos::{
        html::{Input, Select},
        prelude::*,
    };
    use monee_core::ActorId;
    use monee_types::{
        backoffice::actors::{
            actor::Actor, actor_alias::ActorAlias, actor_name::ActorName, actor_type::ActorType,
        },
        shared::errors::UniqueSaveError,
    };
    use web_sys::SubmitEvent;

    use crate::{
        app::components::dialog_form::EntityForm, bind_command,
        leptos_util::local::action::LocalAction, prelude::MoneeError,
    };

    bind_command!(create_actor(actor: Actor) -> ActorId, MoneeError<UniqueSaveError>);

    pub struct CreateActorForm;
    impl EntityForm for CreateActorForm {
        type Id = ActorId;
        fn create_view(
            on_save: impl Fn(ActorId) + Send + Sync + 'static + Copy,
        ) -> impl IntoView + 'static {
            view! { <CreateActorFormC on_save=on_save/> }
        }
    }

    #[component]
    fn CreateActorFormC(on_save: impl Fn(ActorId) + Send + Sync + 'static + Copy) -> impl IntoView {
        let (alias_err, alias_err_msg) = signal::<Option<String>>(None);

        let name_input = NodeRef::<Input>::new();
        let alias_input = NodeRef::<Input>::new();
        let type_select = NodeRef::<Select>::new();
        let get_actor = move || {
            let name: ActorName = name_input.get().unwrap().value().into();
            let alias = match alias_input.get().unwrap().value().as_str() {
                "" => None,
                alias => Some(alias.parse::<ActorAlias>()),
            };
            let actor_type = type_select.get().unwrap().value().parse::<ActorType>();

            if let Some(Err(e)) = &alias {
                alias_err_msg.set(Some(e.to_string()));
            };

            if let (Ok(alias), Ok(actor_type)) = (
                match alias {
                    Some(Ok(alias)) => Ok(Some(alias)),
                    Some(Err(e)) => Err(e),
                    None => Ok(None),
                },
                actor_type,
            ) {
                Some(Actor {
                    name,
                    alias,
                    actor_type,
                })
            } else {
                None
            }
        };

        let action = LocalAction::new(move |actor: Actor| async move {
            let id = create_actor(actor).await?;
            on_save(id);

            Ok(()) as Result<_, MoneeError<UniqueSaveError>>
        });

        let on_submit = {
            let action = action.clone();
            move |e: SubmitEvent| {
                e.prevent_default();
                let actor = get_actor();
                if let Some(actor) = actor {
                    action.dispatch(actor);
                }
            }
        };

        let pending = action.pending();
        let error = action.error();

        view! {
            <form on:submit=on_submit class="grid gap-4">
                <h2>"Create Actor"</h2>

                <input node_ref=name_input required class="bg-slate-800 p-2" type="text" name="name" placeholder="Name" />

                <input node_ref=alias_input class="bg-slate-800 p-2" type="text" name="alias" placeholder="Alias" />
                {move || alias_err.get().map(|msg| view! { <p class="text-red-500">{msg}</p> })}

                {move || if pending.get() { Some(view! { <p>"Saving..."</p> }) } else { None } }
                {move || error().map(|e| match e {
                    MoneeError::App(UniqueSaveError::AlreadyExists(_)) => view! { <p class="text-red-500">"Actor already exists"</p> }.into_any(),
                    MoneeError::Internal(_) => view! { <p class="text-red-500">"Internal error"</p> }.into_any(),
                })}

                <select node_ref=type_select required class="bg-slate-800 p-2" name="type">
                    <option value="n">"Natural"</option>
                    <option value="b">"Bussiness"</option>
                    <option value="f">"Financial Entity"</option>
                </select>
                <button type="submit" class="bg-blue-800 p-2">"Save"</button>
            </form>
        }
    }
}

pub mod create_item {
    use leptos::{html::Input, prelude::*};
    use monee_core::ItemTagId;
    use monee_types::{
        backoffice::item_tags::{item_name::ItemName, item_tag::ItemTag},
        shared::errors::UniqueSaveError,
    };
    use web_sys::SubmitEvent;

    use crate::{
        app::components::dialog_form::EntityForm, bind_command,
        leptos_util::local::action::LocalAction, prelude::MoneeError,
    };

    bind_command!(create_item(item: ItemTag) -> ItemTagId, MoneeError<UniqueSaveError>);

    pub struct CreateItemForm;
    impl EntityForm for CreateItemForm {
        type Id = ItemTagId;
        fn create_view(
            on_save: impl Fn(ItemTagId) + Send + Sync + 'static + Copy,
        ) -> impl IntoView + 'static {
            view! { <CreateItemFormC on_save=on_save/> }
        }
    }

    #[component]
    fn CreateItemFormC(
        on_save: impl Fn(ItemTagId) + Send + Sync + 'static + Copy,
    ) -> impl IntoView {
        let name_ref = NodeRef::<Input>::new();
        let (name_err, name_err_msg) = signal::<Option<String>>(None);

        let action = LocalAction::new(move |item: ItemTag| async move {
            let id = create_item(item).await?;
            on_save(id);

            Ok(()) as Result<_, MoneeError<UniqueSaveError>>
        });

        let on_submit = {
            let action = action.clone();
            move |e: SubmitEvent| {
                e.prevent_default();

                let name = name_ref.get().unwrap().value().parse::<ItemName>();
                match name {
                    Ok(name) => action.dispatch(ItemTag { name }),
                    Err(e) => name_err_msg.set(Some(e.to_string())),
                }
            }
        };

        let pending = action.pending();
        let error = action.error();

        view! {
            <form class="grid gap-4" on:submit=on_submit>
                <h2>"Create Item"</h2>
                <input node_ref=name_ref required class="bg-slate-800 p-2" type="text" name="name" placeholder="Name" />
                {move || name_err.get().map(|msg| view! { <p class="text-red-500">{msg}</p> })}

                {move || if pending.get() { Some(view! { <p>"Saving..."</p> }) } else { None } }
                {move || error().map(|e| match e {
                    MoneeError::App(UniqueSaveError::AlreadyExists(_)) => view! { <p class="text-red-500">"Item already exists"</p> }.into_any(),
                    MoneeError::Internal(_) => view! { <p class="text-red-500">"Internal error"</p> }.into_any(),
                })}

                <button type="submit" class="bg-blue-800 p-2">"Save"</button>
            </form>
        }
    }
}
