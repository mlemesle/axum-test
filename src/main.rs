use axum::{extract::Query, http::StatusCode, routing::get, Router};
use layers::log::LogLayer;
use serde::Deserialize;
use tower::ServiceBuilder;

mod layers;

#[derive(Deserialize)]
struct Q {
    success: bool,
}

async fn do_it(Query(q): Query<Q>) -> Result<(), StatusCode> {
    if q.success {
        Ok(())
    } else {
        Err(StatusCode::NOT_ACCEPTABLE)
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(do_it)).layer(LogLayer);
    // let app = ServiceBuilder::new().layer(LogLayer).service(app);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
