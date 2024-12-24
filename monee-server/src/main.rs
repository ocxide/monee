use axum::{
    http::StatusCode,
    routing::{get, patch, post},
    Router,
};
use monee::shared::domain::context::AppContextBuilder;

mod prelude;

fn main() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(serve());
}

async fn serve() {
    let ctx = AppContextBuilder::default()
        .build()
        .await
        .expect("To build context")
        .setup();

    let app = Router::new()
        .route("/nodes", post(clients::register))
        .route("/sync/guide", get(sync::get_sync_guide))
        .route("/sync", patch(sync::do_sync))
        .route("/sync/report", get(sync::get_host_state))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(ctx);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

mod clients {
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::Json;
    use monee::host;
    use monee::host::nodes::domain::app_id::AppId;
    use monee::host::nodes::domain::app_name::AppName;
    use monee::prelude::*;

    use crate::prelude::CatchInfra;

    #[derive(serde::Deserialize)]
    pub(crate) struct ClientReq {
        name: Option<AppName>,
    }

    #[axum::debug_handler]
    pub async fn register(
        State(ctx): State<AppContext>,
        payload: Option<Json<ClientReq>>,
    ) -> Result<Json<AppId>, StatusCode> {
        println!("register");
        let service: host::nodes::application::register_one::RegisterOne = ctx.provide();
        let name = payload.and_then(|p| p.0.name);

        service
            .run(host::nodes::domain::app_manifest::AppManifest { name })
            .await
            .catch_infra(&ctx)
            .map(Json)
    }
}

mod sync {
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::Json;
    use monee::host::nodes::domain::app_id::AppId;
    use monee::host::sync::domain::host_state::HostState;
    use monee::host::sync::domain::node_changes::NodeChanges;
    use monee::host::sync::domain::{sync_error::SyncError, sync_guide::SyncGuide};
    use monee::prelude::*;
    use monee::shared::domain::errors::UniqueSaveError;

    use crate::prelude::*;

    pub async fn get_sync_guide(
        State(ctx): State<AppContext>,
    ) -> Result<Json<SyncGuide>, StatusCode> {
        println!("get_sync_guide");
        let service: monee::host::sync::application::get_sync_guide::GetSyncGuide = ctx.provide();
        service.run().await.catch_infra(&ctx).map(Json)
    }

    fn get_client_id(headers: &HeaderMap) -> Result<AppId, StatusCode> {
        headers
            .get("X-Node-Id")
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_str()
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .parse()
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }

    #[axum::debug_handler]
    pub async fn do_sync(
        State(ctx): State<AppContext>,
        headers: HeaderMap,
        Json(payload): Json<NodeChanges>,
    ) -> Result<(), StatusCode> {
        println!("do_sync");
        let id = get_client_id(&headers)?;
        let exists_service: monee::host::nodes::application::exists::Exists = ctx.provide();
        if !exists_service.run(id).await.catch_infra(&ctx)? {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let service: monee::host::sync::application::sync_node_changes::SyncNodeChanges =
            ctx.provide();
        service
            .run(id, payload)
            .await
            .catch_infra(&ctx)?
            .map_err(|e| match e {
                SyncError::Save(UniqueSaveError::AlreadyExists(_)) => StatusCode::CONFLICT,
                SyncError::Event(_) => StatusCode::BAD_REQUEST,
            })
    }

    #[axum::debug_handler]
    pub async fn get_host_state(
        State(ctx): State<AppContext>,
        headers: HeaderMap,
    ) -> Result<Json<HostState>, StatusCode> {
        println!("get_host_state");
        let _ = get_client_id(&headers)?;
        let service: monee::host::sync::application::get_host_state::GetHostState = ctx.provide();
        service.run().await.catch_infra(&ctx).map(Json)
    }
}
