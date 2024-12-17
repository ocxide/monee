mod app;

use app::*;
use leptos::{mount::mount_to_body, view};

mod tauri_interop;

mod prelude {
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
    pub enum InternalError {
        Auth,
        Unknown,
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
