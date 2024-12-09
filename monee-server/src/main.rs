use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use cream::context::Context;
use monee::backoffice::currencies::domain::currency::Currency;
use monee::prelude::*;
use monee_core::CurrencyId;

mod prelude;

use prelude::*;

fn main() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(serve());
}

async fn serve() {
    let ctx = monee::shared::domain::context::setup()
        .await
        .expect("To setup context");

    let app = Router::new()
        .route("/currencies", get(list_currencies))
        .with_state(ctx);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn list_currencies(
    State(ctx): State<AppContext>,
) -> Result<Json<Vec<Entity<CurrencyId, Currency>>>, StatusCode> {
    use monee::backoffice::currencies;

    let service: currencies::application::get_all::GetAll = ctx.provide();
    let currencies = service.run().await;

    currencies
        .map(|currencies| Json(currencies.into_iter().map(Into::into).collect()))
        .catch_infra(&ctx)
}
