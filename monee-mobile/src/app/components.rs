pub mod dialog_form;
pub mod host_status_bar {
    use leptos::{prelude::*, IntoView};

    use crate::app_state::{use_host_status, HostStatus};

    #[component]
    pub fn HostStatusBar() -> impl IntoView {
        let host_status = use_host_status();

        let status_bg = move || match host_status.get() {
            Some(HostStatus::Online) => "bg-green-500",
            Some(HostStatus::Offline) => "bg-red-500",
            None => "bg-gray-500",
        };
        let status_text = move || match host_status.get() {
            None => "Could not find host status",
            Some(HostStatus::Online) => "Host is online",
            Some(HostStatus::Offline) => "Host is offline",
        };
        let status_retry = move || matches!(host_status.get(), Some(HostStatus::Offline));

        view! {
            <div class=move || format!("relative w-full py-3 px-2 {}", status_bg())>
                <span>{move || status_text()}"."</span>

                <Show when=move || status_retry()>
                    <span class="text-underline">" Retry"</span>
                    <a href="/" class="absolute top-0 right-0 bottom-0 left-0 w-full h-full"></a>
                </Show>
            </div>
        }
    }
}

pub mod fields {
    pub mod options {
        use crate::prelude::InternalError;
        use leptos::prelude::*;

        pub fn create_options<T, V1>(
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
    }

    pub mod wallet_select {
        use leptos::{html::Select, prelude::*};
        use monee_core::WalletId;

        use super::options::create_options;
        use crate::{bind_command, prelude::InternalError};
        use monee_types::reports::snapshot::snapshot::{Money, Wallet};

        bind_command!(get_all_wallets() -> Vec<(WalletId, (Wallet, Money))>, InternalError);

        #[derive(Default, Clone, Copy)]
        pub struct WalletSelectRef(NodeRef<Select>);

        impl WalletSelectRef {
            pub fn get(&self) -> Option<WalletId> {
                self.0
                    .get_untracked()
                    .and_then(|select| select.value().parse().ok())
            }
        }

        #[component]
        pub fn WalletSelect(#[prop(optional)] node_ref: WalletSelectRef) -> impl IntoView {
            let wallets = LocalResource::new(get_all_wallets);
            let wallets_options = create_options(wallets, |(id, (wallet, money))| {
                view! { <option value={id.to_string()}>{format!("{}: {} {}{}", wallet.name, money.currency.code, money.currency.symbol, money.amount)}</option> }
            });

            view! {
                <select node_ref=node_ref.0 required class="bg-slate-800 p-2" name="wallet_id">
                    {wallets_options}
                </select>
            }
        }
    }

    pub mod amount_input {
        use leptos::{html::Input, prelude::*};
        use monee_core::Amount;

        #[derive(Default, Clone, Copy)]
        pub struct AmountInputRef(NodeRef<Input>);

        impl AmountInputRef {
            pub fn get(&self) -> Option<Amount> {
                self.0
                    .get_untracked()
                    .and_then(|input| input.value().parse().ok())
            }
        }

        #[component]
        pub fn AmountInput(#[prop(optional)] node_ref: AmountInputRef) -> impl IntoView {
            view! {
                <input node_ref=node_ref.0 type="number" required class="bg-slate-800 p-2" name="amount" min="0" />
            }
        }
    }
}
