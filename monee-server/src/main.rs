use axum::{
    routing::{get, patch, post},
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
        .route("/clients", post(clients::register))
        .route("/sync/guide", get(sync::get_sync_guide))
        .route("/sync", patch(sync::do_sync))
        .with_state(ctx);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    use axum::http::{HeaderMap, StatusCode};
    use axum::Json;
    use monee::host::client::domain::client_id::ClientId;
    use monee::host::sync::domain::sync_data::SyncData;
    use monee::host::sync::domain::sync_error::SyncError;
    use monee::host::sync::domain::sync_guide::SyncGuide;
    use monee::prelude::*;
    use monee::shared::domain::errors::UniqueSaveError;

    use crate::prelude::*;

    pub async fn get_sync_guide(
        State(ctx): State<AppContext>,
    ) -> Result<Json<SyncGuide>, StatusCode> {
        let service: monee::host::sync::application::get_sync_guide::GetSyncGuide = ctx.provide();
        service.run().await.catch_infra(&ctx).map(Json)
    }

    #[axum::debug_handler]
    pub async fn do_sync(
        State(ctx): State<AppContext>,
        headers: HeaderMap,
        Json(payload): Json<SyncData>,
    ) -> Result<(), StatusCode> {
        let id: ClientId = headers
            .get("X-Client-Id")
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_str()
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .parse()
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let exists_service: monee::host::client::application::exists::Exists = ctx.provide();
        if !exists_service.run(id).await.catch_infra(&ctx)? {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let service: monee::host::sync::application::do_sync::DoSync = ctx.provide();
        service
            .run(id, payload)
            .await
            .catch_infra(&ctx)?
            .map_err(|e| match e {
                SyncError::Save(UniqueSaveError::AlreadyExists) => StatusCode::CONFLICT,
                SyncError::Event(_) => StatusCode::BAD_REQUEST,
            })
    }
}
