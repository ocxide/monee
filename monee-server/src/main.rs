use axum::{routing::get, Router};

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
        .with_state(ctx);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

mod currencies {
    use axum::{
        extract::State,
        response::{IntoResponse, Response},
    };
    use monee::prelude::*;

    use crate::prelude::*;

    #[axum::debug_handler]
    pub async fn list(State(ctx): State<AppContext>) -> Response {
        use monee::backoffice::currencies;

        let service: currencies::application::get_all::GetAll = ctx.provide();
        let currencies = service.run().await;

        currencies.into_json().catch_infra(&ctx).into_response()
    }
}
