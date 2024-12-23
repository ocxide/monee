pub mod home;
pub mod startup;
pub mod purchase {
    use leptos::{ev::SubmitEvent, prelude::*};
    use leptos_router::hooks::use_navigate;
    use monee_core::{ActorId, Amount, ItemTagId, MoneyError, WalletId};
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
    use web_sys::HtmlOptionElement;

    use crate::{
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
        let (wallet_id, set_wallet_id) = signal::<Option<WalletId>>(None);
        let (item_id, set_item_id) = signal::<Option<ItemTagId>>(None);
        let (actor_ids, set_actor_ids) = signal::<Vec<ActorId>>(Vec::new());
        let (amount, set_amount) = signal::<Amount>(Amount::default());

        let navigate = use_navigate();
        let navigate_back = move || {
            navigate("/home", Default::default());
        };

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

        let items = LocalResource::new(get_all_items);
        let items_options = create_options(items, |item| {
            view! { <option value={item.id.to_string()}>{item.tag.name.to_string()}</option> }
        });

        let actors = LocalResource::new(get_all_actors);
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

        let on_submit = {
            let add_event_action = add_event_action.clone();
            move |e: SubmitEvent| {
                e.prevent_default();

                if let (Some(wallet_id), Some(item_id), actor_ids, amount) = (
                    wallet_id.get(),
                    item_id.get(),
                    actor_ids.get(),
                    amount.get(),
                ) {
                    let event = Event::Buy(Buy {
                        item: item_id,
                        actors: actor_ids.into(),
                        wallet_id,
                        amount,
                    });

                    add_event_action.dispatch(event);
                }
            }
        };

        let output = add_event_action.output();

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

        let on_change =
            move |e: leptos::ev::Targeted<web_sys::Event, web_sys::HtmlSelectElement>| {
                let collection = e.target().selected_options();
                let ids = (0..collection.length())
                    .flat_map(|i| collection.item(i))
                    .map(|el| {
                        el.dyn_into::<HtmlOptionElement>()
                            .unwrap()
                            .value()
                            .parse()
                            .unwrap()
                    })
                    .collect();

                set_actor_ids.set(ids);
            };

        view! {
            <div class="container mx-auto">
                <a href="/home">"Back"</a>

                <form on:submit=on_submit class="grid place-content-center gap-4">
                    <h2>Purchase</h2>

                    <select required class="bg-slate-800 p-2" name="wallet_id" on:change:target=move |e| { set_wallet_id.set(Some(e.target().value().parse().unwrap())) }>
                        {wallets_options}
                    </select>

                    <select required class="bg-slate-800 p-2" name="item_tag_id" on:change:target=move |e| { set_item_id.set(Some(e.target().value().parse().unwrap())) }>
                        {items_options}
                    </select>

                    <select required class="bg-slate-800 p-2" name="actor_ids" multiple on:change:target=on_change>
                        {actors_options}
                    </select>

                    <input required class="bg-slate-800 p-2" type="number" name="amount" placeholder="Amount" on:change:target=move |e| { set_amount.set(e.target().value().parse().unwrap()) } />

                    <button type="submit" class="bg-blue-800 p-2">"Save"</button>

                    {move || {
                        output.with(|state| {
                        match state {
                            None => view! { <p>"Loading..."</p> }.into_any(),
                            Some(Err(e)) => match e {
                                MoneeError::Internal(_) => view! { <p>"Internal Error :("</p> }.into_any(),
                                MoneeError::App(e) => err(e).into_any(),
                            }
                            _ => view! { <p>"Success!"</p> }.into_any(),
                        }
                        })
                    }}
                </form>
            </div>
        }
    }
}
