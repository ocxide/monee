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
            <div class=move || format!("fixed top-0 w-full py-3 px-2 {}", status_bg())>
                <span>{move || status_text()}"."</span>

                <Show when=move || status_retry()>
                    <span class="text-underline">" Retry"</span>
                    <a href="/" class="absolute top-0 right-0 bottom-0 left-0 w-full h-full"></a>
                </Show>
            </div>
        }
    }
}
