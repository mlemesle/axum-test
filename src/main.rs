use axum::Router;
use layers::log::LogLayer;

use tower_http::services::ServeDir;

mod layers;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/", ServeDir::new("jsons/"))
        .layer(LogLayer);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
