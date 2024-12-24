use std::str::FromStr;

use leptos::{
    ev::SubmitEvent,
    html::{Input, Select},
    prelude::*,
};
use leptos_router::hooks::use_navigate;
use monee_core::{ActorId, MoneyError, WalletId};
use monee_types::{
    backoffice::{
        actors::actor::Actor,
        events::{
            apply_event::MoveValueError,
            event::{Buy, Event},
        },
        item_tags::item_tag_node::ItemTagNode,
    },
    reports::snapshot::snapshot::{Money, Wallet},
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlOptionElement, HtmlSelectElement};

use crate::{
    app::{
        components::dialog_form::create_dialog,
        forms::{create_actor::CreateActorForm, create_item::CreateItemForm},
    },
    bind_command,
    leptos_util::local::action::LocalAction,
    prelude::{InternalError, MoneeError},
};

use monee_types::backoffice::events::apply_event::Error as AddEventError;

bind_command!(get_all_wallets() -> Vec<(WalletId, (Wallet, Money))>, InternalError);
bind_command!(get_all_items() -> Vec<ItemTagNode>, InternalError);
bind_command!(add_event(event: Event) -> (), MoneeError<AddEventError>);
bind_command!(get_all_actors() -> Vec<(ActorId, Actor)>, InternalError);

#[component]
pub fn Purchase() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_back = move || {
        navigate("/home", Default::default());
    };

    let wallet_select = NodeRef::<Select>::new();
    let item_select = NodeRef::<Select>::new();
    let actor_select = NodeRef::<Select>::new();
    let amount_input = NodeRef::<Input>::new();

    fn create_options<T, V1>(
        resources: LocalResource<Result<Vec<T>, InternalError>>,
        option: impl Fn(&T) -> V1 + Copy,
    ) -> impl Fn() -> AnyView
    where
        T: 'static,
        V1: IntoView + 'static,
    {
        move || {
            resources.with(|state| {
                state
                    .as_ref()
                    .map(|result| match result.as_deref() {
                        Ok(items) => items.iter().map(option).collect_view().into_any(),
                        Err(_) => view! { <p>"Error"</p> }.into_any(),
                    })
                    .unwrap_or_else(|| view! { <p>"Loading..."</p> }.into_any())
            })
        }
    }

    let wallets = LocalResource::new(get_all_wallets);
    let wallets_options = create_options(wallets, |(id, (wallet, money))| {
        view! { <option value={id.to_string()}>{format!("{}: {} {}{}", wallet.name, money.currency.code, money.currency.symbol, money.amount)}</option> }
    });

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

    let add_event_action = LocalAction::new(move |event: Event| {
        let value = navigate_back.clone();
        async move {
            add_event(event).await?;
            value();

            Ok(()) as Result<_, MoneeError<AddEventError>>
        }
    });

    let get_event = move || {
        fn get_single_value<T: FromStr>(select: HtmlSelectElement) -> Option<T> {
            select.value().parse().ok()
        }
        let wallet_id = wallet_select.get_untracked().and_then(get_single_value);
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
        let amount = amount_input
            .get_untracked()
            .and_then(|input| input.value().parse().ok());

        if let (Some(wallet_id), Some(item_id), actor_ids, Some(amount)) =
            (wallet_id, item_id, actor_ids, amount)
        {
            let event = Event::Buy(Buy {
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

    let on_submit = {
        let add_event_action = add_event_action.clone();
        move |e: SubmitEvent| {
            e.prevent_default();

            let event = get_event();
            if let Some(event) = event {
                add_event_action.dispatch(event);
            }
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

    let output = add_event_action.output();
    let pending = add_event_action.pending();

    view! {
        <div class="container mx-auto">
            <a href="/home">"Back"</a>

            {actor_form}
            {item_form}

            <form on:submit=on_submit class="grid place-content-center gap-4">
                <h2>Purchase</h2>

                <select node_ref=wallet_select required class="bg-slate-800 p-2" name="wallet_id">
                    {wallets_options}
                </select>

                <div class="flex gap-x-4">
                    <select node_ref=item_select required class="bg-slate-800 p-2 flex-1" name="item_tag_id">
                        {items_options}
                    </select>

                    <button type="button" class="bg-blue-800 p-2 rounded-full" on:click=move |_| open_item_form()>+</button>
                </div>

                <div class="flex gap-x-4">
                    <select node_ref=actor_select class="bg-slate-800 p-2" name="actor_ids" multiple>
                        {actors_options}
                    </select>

                    <button type="button" class="bg-blue-800 p-2 rounded-full" on:click=move |_| open_actor_form()>+</button>
                </div>

                <input node_ref=amount_input required class="bg-slate-800 p-2" type="number" name="amount" placeholder="Amount" />

                <button type="submit" class="bg-blue-800 p-2">"Save"</button>

                <Show when=move || pending.get()>
                    <p>"Saving..."</p>
                </Show>

                <Show when={ let action = add_event_action.clone(); move || action.is_err() }>
                {move || output.with(|state| {
                    match state {
                    Some(Err(MoneeError::App(e))) => Some(err(e).into_any()),
                    Some(Err(MoneeError::Internal(_))) => Some(view! { <p>"Internal error :("</p> }.into_any()),
                    _ => None,
                    }
                })}
                </Show>
            </form>
        </div>
    }
}
