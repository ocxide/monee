use axum::{
    routing::{get, post},
    Router,
};

mod prelude;

fn main() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(serve());
}

async fn serve() {
    let ctx = monee::shared::domain::context::setup()
        .await
        .expect("To setup context");

    let app = Router::new()
        .route("/currencies", get(currencies::list))
        .route("/currencies", post(currencies::create_one))
        .route("/clients", post(clients::register))
        .route("/sync/guide", get(sync::get_sync_guide))
        .with_state(ctx);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

mod currencies {
    use axum::{
        extract::State,
        response::{IntoResponse, Response},
        Json,
    };
    use monee::backoffice::currencies::{self, domain::currency::Currency};
    use monee::prelude::*;

    use crate::prelude::*;

    #[axum::debug_handler]
    pub async fn list(State(ctx): State<AppContext>) -> Response {
        let service: currencies::application::get_all::GetAll = ctx.provide();
        let currencies = service.run().await;

        currencies.into_json().catch_infra(&ctx).into_response()
    }

    #[axum::debug_handler]
    pub async fn create_one(
        State(ctx): State<AppContext>,
        Json(payload): Json<Currency>,
    ) -> Response {
        let service: currencies::application::save_one::SaveOne = ctx.provide();
        service
            .run(payload)
            .await
            .catch_app()
            .catch_infra(&ctx)
            .into_response()
    }
}

mod clients {
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::Json;
    use monee::host::client::domain::client_id::ClientId;
    use monee::host::client::domain::client_name::ClientName;
    use monee::prelude::*;

    use monee::host::client;

    use crate::prelude::CatchInfra;

    #[derive(serde::Deserialize)]
    pub(crate) struct ClientReq {
        name: Option<ClientName>,
    }

    #[axum::debug_handler]
    pub async fn register(
        State(ctx): State<AppContext>,
        Json(payload): Json<Option<ClientReq>>,
    ) -> Result<Json<ClientId>, StatusCode> {
        let service: client::application::register_one::RegisterOne = ctx.provide();
        let name = payload.and_then(|p| p.name);

        service
            .run(client::domain::client::Client { name })
            .await
            .catch_infra(&ctx)
            .map(Json)
    }
}

mod sync {
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::Json;
    use monee::host::sync::domain::sync_guide::SyncGuide;
    use monee::prelude::*;

    use crate::prelude::*;

    pub async fn get_sync_guide(
        State(ctx): State<AppContext>,
    ) -> Result<Json<SyncGuide>, StatusCode> {
        let service: monee::host::sync::application::get_sync_guide::GetSyncGuide = ctx.provide();
        service.run().await.catch_infra(&ctx).map(Json)
    }
}
