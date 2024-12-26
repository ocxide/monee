use std::marker::PhantomData;

use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::hooks::use_navigate;
use monee_core::MoneyError;
use monee_types::backoffice::events::apply_event::Error as AddEventError;
use monee_types::backoffice::events::apply_event::MoveValueError;
use monee_types::backoffice::events::event::Event;
use web_sys::SubmitEvent;

use crate::bind_command;
use crate::leptos_util::signal::AppWithErr;
use crate::{leptos_util::local::action::local_action, prelude::MoneeError};

pub mod purchase {
    use std::str::FromStr;

    use leptos::{html::Select, prelude::*};
    use monee_core::ActorId;
    use monee_types::backoffice::{
        actors::actor::Actor,
        events::event::{Event, Purchase},
        item_tags::item_tag_node::ItemTagNode,
    };
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlOptionElement, HtmlSelectElement};

    use crate::{
        app::{
            components::{
                dialog_form::create_dialog,
                fields::{
                    amount_input::{AmountInput, AmountInputRef},
                    options::create_options,
                    wallet_select::{WalletSelect, WalletSelectRef},
                },
            },
            forms::{create_actor::CreateActorForm, create_item::CreateItemForm},
        },
        bind_command,
        prelude::InternalError,
    };

    use super::EventForm;

    bind_command!(get_all_items() -> Vec<ItemTagNode>, InternalError);
    bind_command!(get_all_actors() -> Vec<(ActorId, Actor)>, InternalError);

    pub struct PurchaseForm;
    impl EventForm for PurchaseForm {
        const TITLE: &'static str = "Purchase";
        fn create() -> (
            impl IntoView,
            impl IntoView,
            impl Fn() -> Option<Event> + 'static + Copy,
        ) {
            purchase()
        }
    }

    pub fn purchase() -> (
        impl IntoView,
        impl IntoView,
        impl Fn() -> Option<Event> + 'static + Copy,
    ) {
        let wallet_select = WalletSelectRef::default();
        let item_select = NodeRef::<Select>::new();
        let actor_select = NodeRef::<Select>::new();
        let amount_input = AmountInputRef::default();

        let (item_refresh, set_item_refresh) = signal(());
        let items = LocalResource::new(move || async move {
            item_refresh.get();
            get_all_items().await
        });
        let items_options = create_options(items, |item| {
            view! { <option value={item.id.to_string()}>{item.tag.name.to_string()}</option> }
        });

        let (actor_refresh, set_actor_refresh) = signal(());
        let actors = LocalResource::new(move || async move {
            actor_refresh.get();
            get_all_actors().await
        });
        let actors_options = create_options(actors, |(id, actor)| {
            let msg = match &actor.alias {
                Some(alias) => format!("{} - {}", alias, actor.name),
                None => actor.name.to_string(),
            };
            view! { <option value={id.to_string()}>{msg}</option> }
        });

        let get_event = move || {
            fn get_single_value<T: FromStr>(select: HtmlSelectElement) -> Option<T> {
                select.value().parse().ok()
            }
            let wallet_id = wallet_select.get();
            let item_id = item_select.get_untracked().and_then(get_single_value);
            let actor_ids = actor_select
                .get_untracked()
                .map(|select| {
                    let collection = select.selected_options();
                    let ids: Vec<_> = (0..collection.length())
                        .flat_map(|i| collection.item(i))
                        .map(|el| {
                            el.dyn_into::<HtmlOptionElement>()
                                .unwrap()
                                .value()
                                .parse::<ActorId>()
                                .unwrap()
                        })
                        .collect();
                    ids
                })
                .unwrap_or_default();
            let amount = amount_input.get();

            if let (Some(wallet_id), Some(item_id), actor_ids, Some(amount)) =
                (wallet_id, item_id, actor_ids, amount)
            {
                let event = Event::Purchase(Purchase {
                    item: item_id,
                    actors: actor_ids.into(),
                    wallet_id,
                    amount,
                });
                Some(event)
            } else {
                None
            }
        };

        let (open_actor_form, actor_form) = create_dialog::<CreateActorForm>(move |_| {
            set_actor_refresh.set(());
        });

        let (open_item_form, item_form) = create_dialog::<CreateItemForm>(move |item_id| {
            item_select
                .get()
                .unwrap()
                .set_value(item_id.to_string().as_str());

            set_item_refresh.set(());
        });

        let fragment = view! {
            {actor_form}
            {item_form}
        };
        let form = view! {
            <>
                <WalletSelect node_ref=wallet_select />

                <div class="flex gap-x-4">
                    <select node_ref=item_select required class="bg-slate-800 p-2 flex-1" name="item_tag_id">
                        {items_options}
                    </select>

                    <button type="button" class="bg-blue-800 p-2 rounded-full" on:click=move |_| open_item_form()>+</button>
                </div>

                <div class="flex gap-x-4">
                    <select node_ref=actor_select class="bg-slate-800 p-2 w-full" name="actor_ids" multiple>
                        {actors_options}
                    </select>

                    <button type="button" class="bg-blue-800 p-2 rounded-full" on:click=move |_| open_actor_form()>+</button>
                </div>

                <AmountInput node_ref=amount_input />
            </>
        };

        (form, fragment, get_event)
    }
}

pub mod move_value {
    use leptos::prelude::*;
    use monee_types::backoffice::events::event::{Event, MoveValue};

    use crate::app::components::fields::{
        amount_input::{AmountInput, AmountInputRef},
        wallet_select::{WalletSelect, WalletSelectRef},
    };

    use super::EventForm;

    pub struct MoveValueForm;

    impl EventForm for MoveValueForm {
        const TITLE: &'static str = "Move Value";
        fn create() -> (
            impl leptos::IntoView,
            impl leptos::IntoView,
            impl Fn() -> Option<Event> + 'static + Copy,
        ) {
            move_value()
        }
    }

    fn move_value() -> (
        impl IntoView,
        impl IntoView,
        impl Fn() -> Option<Event> + 'static + Copy,
    ) {
        let from_ref = WalletSelectRef::default();
        let to_ref = WalletSelectRef::default();
        let amount_input = AmountInputRef::default();

        let form = view! {
            <>
                <WalletSelect node_ref=from_ref />
                <WalletSelect node_ref=to_ref />
                <AmountInput node_ref=amount_input />
            </>
        };

        let get_event = move || {
            if let (Some(from), Some(to), Some(amount)) =
                (from_ref.get(), to_ref.get(), amount_input.get())
            {
                let event = Event::MoveValue(MoveValue { from, to, amount });
                Some(event)
            } else {
                None
            }
        };

        (form, (), get_event)
    }
}

pub trait EventForm {
    const TITLE: &'static str;
    fn create() -> (
        impl IntoView,
        impl IntoView,
        impl Fn() -> Option<Event> + 'static + Copy,
    );
}

bind_command!(add_event(event: Event) -> (), MoneeError<AddEventError>);

#[component]
pub fn EventPageForm<F: EventForm>(#[prop(optional)] _f: PhantomData<F>) -> impl IntoView {
    let (form, fragment, get_event) = F::create();

    let navigate = use_navigate();
    let navigate_back = move || {
        navigate("/home", Default::default());
    };

    let (action, dispatch) = local_action(move |event: Event| {
        let value = navigate_back.clone();
        async move {
            add_event(event).await?;
            value();

            Ok(()) as Result<_, MoneeError<AddEventError>>
        }
    });

    let on_submit = move |e: SubmitEvent| {
        e.prevent_default();

        let event = get_event();
        if let Some(event) = event {
            dispatch.dispatch(event);
        }
    };

    let err = |error: &AddEventError| {
        let msg = match error {
            AddEventError::MoveValue(MoveValueError::CurrenciesNonEqual) => {
                "Currencies are not equal".to_string()
            }
            AddEventError::MoveValue(MoveValueError::WalletNotFound(wallet_id)) => {
                format!("Wallet {wallet_id} not found")
            }

            AddEventError::Apply(monee_core::Error::Wallet(MoneyError::CannotSub)) => {
                "Cannot deduct".to_string()
            }

            _ => "Error".to_string(),
        };

        view! { <p>{msg}</p> }
    };

    view! {
        <div class="py-8 h-full">
            <a href="/home" class="underline">"Back"</a>

            {fragment}

            <div class="grid place-content-center h-full">
                <form on:submit=on_submit class="grid place-content-center gap-4 h-full">
                    <h2 class="text-2xl">{F::TITLE}</h2>

                    {form}

                    <button type="submit" class="bg-blue-800 p-2">"Save"</button>

                    <Show when=move || action.pending()>
                        <p>"Saving..."</p>
                    </Show>

                    <Show when=move || action.is_err()>
                        {move || action.with(|state| {
                            match state {
                            Some(Err(MoneeError::App(e))) => Some(err(e).into_any()),
                            Some(Err(MoneeError::Internal(_))) => Some(view! { <p>"Internal error :("</p> }.into_any()),
                            _ => None,
                            }
                        })}
                    </Show>
                </form>
            </div>
        </div>
    }
}
