use axum::Router;
use layers::{count_char::CountCharLayer, log::LogLayer};

use tower_http::services::ServeDir;

mod layers;
mod simple_future;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/", ServeDir::new("jsons/"))
        .layer(CountCharLayer::new('f'))
        .layer(LogLayer);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
