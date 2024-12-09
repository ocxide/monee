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
